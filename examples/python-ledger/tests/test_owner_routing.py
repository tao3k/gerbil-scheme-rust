"""Real installed Scheme sessions, stateless routing and owner-batched native values."""
import concurrent.futures
import collections
import os
import json
import unittest
import gerbil_ledger

class OwnerRoutingTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.library = gerbil_ledger.open_library(owners=4, dispatch_workers=2)
        cls.states = cls.library.vm_states

    @classmethod
    def tearDownClass(cls):
        assert cls.library.vm_states == cls.states
        cls.library.collect()
        path = cls.library.runtime_directory
        cls.library.close()
        assert cls.library.shutdown_clean and cls.library.shutdown_roots == [0] * 4
        assert not os.path.exists(path)

    def test_owner_profile_is_opt_in_bounded_and_drained(self):
        self.library.owner_profile()
        root=self.library.call(("echo", 23));root.close()
        self.assertEqual(json.loads(self.library.owner_profile())["spans"], [])
        self.library.set_owner_profiling(True)
        try:
            roots=self.library.call_many([("echo", i) for i in range(8)], dispatch="direct")
            self.assertEqual(self.library.project_many(roots, release=True), list(range(8)))
            trace=json.loads(self.library.owner_profile())
            self.assertEqual(trace["dropped"], 0)
            self.assertEqual(sum(row["owner"] is not None for row in trace["spans"]), 16)
            self.assertEqual({row["owner"] for row in trace["spans"] if row["owner"] is not None}, set(range(4)))
            for row in trace["spans"]:
                if row["owner"] is None:continue
                self.assertLessEqual(row["queued_ns"], row["start_ns"])
                self.assertLessEqual(row["start_ns"], row["end_ns"])
                self.assertFalse(row["failed"])
                if row["kind"] == "batch-call":
                    self.assertLessEqual(row["stages_ns"][0], row["stages_ns"][1])
                    self.assertGreater(row["stages_ns"][1], 0)
                self.assertIsNotNone(row["diagnostics_delta"])
            self.assertEqual(json.loads(self.library.owner_profile())["spans"], [])
        finally:
            self.library.set_owner_profiling(False)

    def test_bounded_admission_preserves_routing_and_order(self):
        for mode in ("direct", "tokio", "rayon", "rayon-blocking", "tokio-pipeline"):
            for window in (3,16):
                roots=self.library.call_many([("echo", i) for i in range(129)], dispatch=mode, admission_window=window)
                first=roots[0].owner
                self.assertEqual([root.owner for root in roots], [(first+i)%4 for i in range(129)])
                self.assertEqual(self.library.project_many(roots, release=True), list(range(129)))
        for window in (1, 3, 16, 127, 128):
            roots, ready=self.library.call_many_profiled([("echo", i) for i in range(33)], dispatch="direct", admission_window=window)
            self.assertEqual(self.library.project_many(roots, release=True), list(range(33)))
            self.assertEqual(len(ready),33)
        for window in (0,129):
            with self.assertRaises(RuntimeError):
                self.library.call_many_profiled([("echo",0)], admission_window=window)

    def test_prepared_native_snapshots_are_immutable_and_fresh(self):
        data=[1,{"bytes":b"\0\xff","big":2**180}]
        request=self.library.prepare_request(("echo",data))
        data[0]=9
        roots=self.library.call_many([request]*129,dispatch="direct",admission_window=3)
        self.assertEqual(self.library.project_many(roots,release=True),[[1,{"bytes":b"\0\xff","big":2**180}]]*129)
        a=self.library.call(request);b=self.library.call(request)
        self.assertFalse(a.is_identical(b));self.library.collect()
        self.assertEqual(a.to_python(),[1,{"bytes":b"\0\xff","big":2**180}])
        a.close();b.close()
        for mode in ("tokio","rayon","rayon-blocking","tokio-pipeline"):
            roots=self.library.call_many([self.library.prepare_request(request)]*17,dispatch=mode,admission_window=3)
            self.assertEqual(len(self.library.project_many(roots,release=True)),17)

    def test_prepared_references_cycles_and_composed_depth(self):
        root=self.library.call(("echo",42))
        with self.assertRaises(ValueError):self.library.prepare_request(("echo",[root]))
        root.close()
        cycle=[];cycle.append(cycle)
        with self.assertRaises(ValueError):self.library.prepare_request(("echo",cycle))
        request=self.library.prepare_request(0)
        for _ in range(128):request=self.library.prepare_request((request,))
        with self.assertRaises(ValueError):self.library.prepare_request((request,))
        for _ in range(200):request=self.library.prepare_request(request)

    def test_prepared_requests_keep_full_scheme_execution(self):
        for owner in range(4):
            for expression in ("(let ((foldl #f)) (ledger-sum 20 22))", "(begin (defrule (twice x) (+ x x)) (twice 21))"):
                result=self.library.session(owner).call(self.library.prepare_request(("dsl",expression)))
                self.assertEqual(result.to_python(),42);result.close()
            function=self.library.session(owner).call(self.library.prepare_request(("saved-continuation",)))
            self.library.collect()
            for arg in (1,42):
                result=function.apply(arg);self.assertEqual(result.to_python(),65536+arg);result.close()
            function.close()
            with self.assertRaises(RuntimeError):self.library.session(owner).call(self.library.prepare_request(("unknown",)))
            result=self.library.session(owner).call(self.library.prepare_request(("echo",42)))
            self.assertEqual(result.to_python(),42);result.close()

    def test_pipeline_errors_drain_and_preserve_native_roots(self):
        for window in (1,3,16,128):
            roots,ready=self.library.call_many_profiled([("echo",i) for i in range(257)],dispatch="tokio-pipeline",admission_window=window)
            self.assertEqual(self.library.project_many(roots,release=True),list(range(257)))
            self.assertEqual(len(ready),257)
        with self.assertRaises(RuntimeError):
            self.library.call_many([("echo",1),("unknown",),("echo",3)]*16,dispatch="tokio-pipeline",admission_window=16)
        roots=self.library.call_many([("echo",i) for i in range(64)],dispatch="tokio-pipeline",admission_window=16)
        self.assertEqual(self.library.project_many(roots,release=True),list(range(64)))

    def test_pipeline_global_and_owner_credit_bounds(self):
        for window in (1,3,16):
            self.library.owner_profile();self.library.set_owner_profiling(True)
            try:
                roots=self.library.call_many([("echo",i) for i in range(129)],dispatch="tokio-pipeline",admission_window=window)
                self.assertEqual(self.library.project_many(roots,release=True),list(range(129)))
                trace=json.loads(self.library.owner_profile())
                self.assertEqual(trace["dropped"],0)
                spans=[row for row in trace["spans"] if row["kind"]=="batch-call"]
                self.assertEqual(len(spans),129)
                for owner in (None,0,1,2,3):
                    selected=spans if owner is None else [row for row in spans if row["owner"]==owner]
                    events=sorted([(row["queued_ns"],1) for row in selected]+[(row["end_ns"],-1) for row in selected])
                    active=maximum=0
                    for _,change in events:active+=change;maximum=max(maximum,active)
                    self.assertEqual(active,0)
                    self.assertLessEqual(maximum,window if owner is None else (window+3)//4)
            finally:
                self.library.set_owner_profiling(False)

    def test_stateless_cursor_spans_individual_and_batch_calls(self):
        roots = [self.library.call_stateless(("echo", i)) for i in range(5)]
        first = roots[0].owner
        self.assertEqual([v.owner for v in roots], [(first+i) % 4 for i in range(5)])
        more = self.library.call_many([("echo", i) for i in range(7)], dispatch="direct")
        self.assertEqual([v.owner for v in more], [(first+5+i) % 4 for i in range(7)])
        self.assertEqual(self.library.project_many(roots+more, release=True), list(range(5))+list(range(7)))

    def test_stateful_sessions_keep_vm_state_and_default_call_owner(self):
        sessions = [self.library.session(i) for i in range(4)]
        totals = self.library.project_many([s.call(("total",)) for s in sessions], release=True)
        result = sessions[2].call(("append", 73))
        self.assertEqual(result.owner, 2)
        result.close()
        later = self.library.project_many([s.call(("total",)) for s in sessions], release=True)
        self.assertEqual(later, [n+73 if i==2 else n for i,n in enumerate(totals)])
        same = self.library.session(2).call(("total",))
        self.assertEqual(same.to_python(), later[2]); same.close()
        default = self.library.call(("echo", 42))
        self.assertEqual(default.owner, 0); default.close()
        before = sessions[3].call(("entries",))
        previous = before.to_python(); before.close()
        roots = sessions[3].call_many([("append", i) for i in range(8)] + [("entries",)])
        self.assertEqual([v.owner for v in roots], [3] * 9)
        output = self.library.project_many(roots, release=True)
        self.assertEqual(output[:-1], list(range(len(previous)+1, len(previous)+9)))
        self.assertEqual(output[-1], previous+list(range(8)))
        with self.assertRaises(ValueError): self.library.session(4)

    def test_references_require_affinity_and_reject_before_stateful_batch(self):
        left, right = self.library.session(0), self.library.session(1)
        value = right.value([2**150, b"\x00\xff", "λ"])
        for submit in (self.library.call_stateless, lambda x:self.library.call_many([x])):
            with self.assertRaises(RuntimeError): submit(("echo", (value,)))
        before = left.call(("total",)); expected=before.to_python();before.close()
        with self.assertRaises(RuntimeError): left.call_many([("append", 99), ("echo", value)])
        after = left.call(("total",)); self.assertEqual(after.to_python(), expected);after.close()
        result = right.call(("echo", value));self.assertTrue(value.is_identical(result))
        result.close();value.close()

    def test_bulk_order_duplicates_types_and_idempotent_retirement(self):
        data = [None, False, 2**170, -(2**151), 1.25, "λ", b"\x00\xff", [1,[2]], {"x":3}]
        roots = [self.library.session(i%4).call(("echo", v)) for i,v in enumerate(data)]
        roots.reverse();data.reverse()
        values = [roots[0], *roots, roots[0]]
        self.assertEqual(self.library.project_many(values), [data[0], *data, data[0]])
        self.assertEqual(self.library.project_many(values, release=True), [data[0], *data, data[0]])
        self.library.release_many(values);self.library.release_many(values)
        for root in roots:
            with self.assertRaises(RuntimeError):root.to_python()
        self.assertEqual(self.library.project_many([], release=True), [])
        self.library.release_many([])
        with self.assertRaises(ValueError):self.library.project_many([roots[0]] * 4097)
        live=self.library.call(("echo",91))
        with self.assertRaises(TypeError):self.library.release_many([live, 42])
        self.assertEqual(live.to_python(),91);live.close()

    def test_projection_and_release_across_native_windows(self):
        roots=self.library.call_many([("echo", (i, 2**(130+i%3))) for i in range(257)], dispatch="direct")
        roots.reverse()
        expected=[[i,2**(130+i%3)] for i in reversed(range(257))]
        self.assertEqual(self.library.project_many(roots+[roots[0]],release=True),expected+[expected[0]])
        self.library.release_many(roots)
        for root in roots:
            with self.assertRaises(RuntimeError):root.to_python()

    def test_projection_failure_keeps_graphs_procedures_and_continuations(self):
        plain=self.library.session(0).call(("echo",42))
        cycle=self.library.session(1).call(("cycle",))
        procedure=self.library.session(2).call(("procedure",))
        continuation=self.library.session(3).call(("saved-continuation",))
        roots=[plain,cycle,procedure,continuation]
        with self.assertRaises(RuntimeError):self.library.project_many(roots,release=True)
        self.library.collect()
        self.assertEqual(plain.to_python(),42)
        child=cycle.elements()[0];self.assertTrue(cycle.is_identical(child));child.close()
        for function, argument, expected in ((procedure,41,42),(continuation,1,65537),(continuation,42,65578)):
            result=function.apply(argument)
            self.assertEqual(result.to_python(),expected);result.close()
        self.library.release_many(roots)
        for owner in range(4):
            with self.assertRaises(RuntimeError):self.library.session(owner).call(("unknown",))
        good=self.library.call_stateless(("echo",42));self.assertEqual(good.to_python(),42);good.close()

    def test_foreign_library_validation_does_not_consume_local_values(self):
        other=gerbil_ledger.open_library(owners=1)
        try:
            local=self.library.call(("echo",1));foreign=other.call(("echo",2))
            for operation in (self.library.release_many,lambda roots:self.library.project_many(roots,release=True)):
                with self.assertRaises(ValueError):operation([local,foreign])
            self.assertEqual(local.to_python(),1);self.assertEqual(foreign.to_python(),2)
            local.close();foreign.close()
        finally:
            other.close();self.assertTrue(other.shutdown_clean)

    def test_concurrent_stateless_callers_and_bulk_collection(self):
        def task(i):
            root=self.library.call_stateless(("echo",i));owner=root.owner
            value=self.library.project_many([root],release=True)[0]
            return owner,value
        with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
            results=list(pool.map(task,range(64)))
        self.assertEqual([v for _,v in results],list(range(64)))
        self.assertEqual(collections.Counter(owner for owner,_ in results),dict.fromkeys(range(4),16))

if __name__ == "__main__": unittest.main()
