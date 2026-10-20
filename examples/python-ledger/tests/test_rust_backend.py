"""Installed default Rust dispatch enters real fixed-owner Scheme VMs."""
import concurrent.futures
import os
import unittest
import gerbil_ledger
from gerbil_ledger._core import scheme_initialization_attempted

class RustBackendTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.library=gerbil_ledger.open_library()
    @classmethod
    def tearDownClass(cls):
        retained=cls.library.call(("echo","retained until close"))
        path=cls.library.runtime_directory
        cls.library.close();cls.library.close()
        assert cls.library.shutdown_clean
        assert cls.library.shutdown_roots==[0]*4
        assert not os.path.exists(path)
        with cls().assertRaises(RuntimeError):retained.to_python()
        retained.close()
    def call(self,*request,owner=0):
        value=self.library.call_on(owner,request)
        try:return value.to_python()
        finally:value.close()
    def test_default_is_four_distinct_scheme_owners(self):
        self.assertTrue(scheme_initialization_attempted())
        self.assertEqual(self.library.backend,"rust")
        self.assertEqual(self.library.vm_count,4)
        self.assertEqual(len(set(self.library.vm_states)),4)
        self.assertEqual(self.library.execution_model,"tokio-dispatch-fixed-scheme-owners")
        self.assertEqual(self.library.active_processors,1)
    def test_native_types_and_finite_collections(self):
        for value in [None,False,True,0,2**150,-2**137,1.25,"λ",b"\x00\xff",[1,2],{"x":3}]:
            with self.subTest(value=value):self.assertEqual(self.call("echo",value),value)
    def test_state_is_separate_per_owner(self):
        for owner in range(4):
            self.call("append",owner+1,owner=owner)
            self.assertEqual(self.call("total",owner=owner),owner+1)
    def test_closures_gc_and_continuations(self):
        for owner in range(4):
            self.assertEqual(self.call("continuation",owner=owner),42)
            procedure=self.library.call_on(owner,("procedure",))
            self.call("gc",owner=owner)
            result=procedure.apply(2**130)
            self.assertEqual(result.to_python(),2**130+1)
            result.close();procedure.close()
            continuation=self.library.call_on(owner,("saved-continuation",))
            for argument in (1,42):
                self.call("gc",owner=owner)
                resumed=continuation.apply(argument)
                self.assertEqual(resumed.to_python(),65536+argument)
                resumed.close()
            continuation.close()
    def test_graph_identity_cycles_and_gc(self):
        parent=self.library.call(("shared",));left,right=parent.elements();parent.close()
        self.call("gc");self.assertTrue(left.is_identical(right));left.close();right.close()
        parent=self.library.call(("cycle",));child=parent.elements()[0]
        with self.assertRaises(RuntimeError):parent.to_python()
        self.call("gc");self.assertTrue(parent.is_identical(child));parent.close();child.close()
    def test_exceptions_recover_and_foreign_owner_handles_reject(self):
        for owner in range(4):
            with self.assertRaises(RuntimeError):self.call("unknown",owner=owner)
            self.assertEqual(self.call("echo",42,owner=owner),42)
        value=self.library.call_on(1,("echo",91))
        with self.assertRaises(RuntimeError):self.library.call_on(0,("echo",value))
        self.assertEqual(value.to_python(),91);value.close()
    def test_concurrent_callers_and_same_vm_batch_modes(self):
        with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
            self.assertEqual(list(pool.map(lambda v:self.call("echo",v,owner=v%4),range(64))),list(range(64)))
        for mode in ("direct","tokio","rayon","rayon-blocking"):
            roots,ns=self.library.call_many_profiled([("echo",i) for i in range(128)],dispatch=mode)
            self.assertEqual([v.to_python() for v in roots],list(range(128)))
            self.assertEqual(len(ns),128);self.assertTrue(all(n>0 for n in ns))
            for value in roots:value.close()
    def test_actor_work_and_fair_completion(self):
        items=tuple((i,tuple((j,3) for j in range(48)),True) for i in range(32))
        for owner in range(4):
            output=self.call("actors",items,1,owner=owner)
            self.assertEqual([r[0] for r in output],list(range(32)))
            self.assertEqual({r[1] for r in output},{3384})
            self.assertEqual(sorted(r[2] for r in output),list(range(1,33)))
            self.assertEqual({r[3] for r in output},{0,1,2,3})
    def test_full_expander_and_independent_native_bundle_images(self):
        from pathlib import Path
        identities=[]
        for owner in range(4):
            self.assertEqual(self.call("dsl","(let ((foldl #f)) (ledger-sum 20 22))",owner=owner),42)
            with self.assertRaises(RuntimeError):self.call("dsl",'(error "injected Scheme error")',owner=owner)
            self.assertEqual(self.call("dsl","(+ 20 22)",owner=owner),42)
            root=Path(self.library.runtime_directory)/f"owner-{owner}-modules"
            parts=list(root.rglob("*.o1"))
            inode_ids={(p.stat().st_dev,p.stat().st_ino) for p in parts}
            self.assertGreater(len(parts),len(inode_ids))
            for prior in identities:self.assertTrue(prior.isdisjoint(inode_ids))
            identities.append(inode_ids)
    def test_explicit_gc_after_root_release(self):
        for usage in self.library.collect():self.assertEqual(usage[3],0)

if __name__=="__main__":unittest.main()
