"""Native code leases cover the owner image and SDK-managed dynamic modules."""
import argparse
import sys
import ctypes
import gc
import os
import time
import signal
import unittest
import gerbil_ledger

parser = argparse.ArgumentParser(add_help=False)
parser.add_argument("--image-policy", choices=("retained", "qualified-release"), default="retained")
control, remainder = parser.parse_known_args()
sys.argv = [sys.argv[0], *remainder]
loader = ctypes.CDLL(None)
loader._dyld_image_count.restype = ctypes.c_uint32
loader._dyld_get_image_name.argtypes = (ctypes.c_uint32,)
loader._dyld_get_image_name.restype = ctypes.c_char_p

def images():
    prefix = f"/gerbil-owners-{os.getpid()}-".encode()
    return [name.decode() for i in range(loader._dyld_image_count())
            if (name := loader._dyld_get_image_name(i)) and prefix in name]

class ImageRetirementTests(unittest.TestCase):
    def open_expanders(self):
        if self.release:
            from gerbil_ledger.qualification import open_qualified_retirement_library
            library = open_qualified_retirement_library()
        else:
            library = gerbil_ledger.open_library()
        self.assertEqual(library.image_release_authorized, self.release)
        for owner in range(4):
            value = library.call_on(owner, ("dsl", "(+ 20 22)"))
            self.assertEqual(value.to_python(), 42)
            value.close()
        self.assertEqual(len(images()) - self.initial_images, 8)
        return library

    def setUp(self):
        self.initial_images = len(images())
        self.release = control.image_policy == "qualified-release"
        self.state = gerbil_ledger._core.ImageMappingState

    def test_explicit_lease_retains_sdk_module_until_last_close(self):
        library = self.open_expanders()
        first = library.lease_code_on(0)
        second = library.lease_code_on(0)
        library.close()
        self.assertTrue(library.shutdown_clean)
        self.assertEqual(library.shutdown_roots, [0]*4)
        self.assertEqual(first.mapping.state, self.state.RetainedByLeases if self.release else self.state.RetainedByPolicy)
        self.assertEqual(first.mapping.code_leases, 2)
        self.assertGreaterEqual(first.mapping.pending_module_closes, 1)
        # Four base images stay by default policy. The qualified program keeps
        # only owner zero's base and expander while its code is explicitly leased.
        expected = 2 if self.release else 5
        print("MODULE-LEASE after-close", images(), flush=True)
        self.assertEqual(len(images()) - self.initial_images, expected)
        second.close(); second.close()
        self.assertEqual(first.mapping.code_leases, 1)
        first.close(); first.close()
        expected_state = self.state.Released if self.release else self.state.RetainedByPolicy
        self.assertEqual(first.mapping.state, expected_state)
        self.assertEqual(first.mapping.code_leases, 0)
        self.assertEqual(len(images()) - self.initial_images, 0 if self.release else 4)
        with self.assertRaises(RuntimeError):
            library.lease_code_on(0)

    @unittest.skipUnless(hasattr(os, "fork"), "requires native fork")
    def test_inherited_code_lease_rejects_before_host_locks(self):
        library = gerbil_ledger.open_library(owners=1)
        self.addCleanup(library.close)
        lease = library.lease_code_on(0)
        self.addCleanup(lease.close)
        child = os.fork()
        if child == 0:
            try:
                for operation in (lambda: lease.mapping, lease.close,
                                  lambda: library.lease_code_on(0)):
                    try:
                        operation()
                    except RuntimeError as error:
                        if "fork" not in str(error):
                            os._exit(2)
                    else:
                        os._exit(3)
                # Release the inherited wrapper: no inherited mutex or native
                # destructor may run in the child. The parent guard stays live.
                del operation
                self._cleanups.clear()
                del lease
                gc.collect()
                os._exit(0)
            except BaseException:
                os._exit(4)
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            finished, status = os.waitpid(child, os.WNOHANG)
            if finished:
                self.assertEqual(os.waitstatus_to_exitcode(status), 0)
                self.assertEqual(lease.mapping.code_leases, 1)
                return
            time.sleep(.01)
        os.kill(child, signal.SIGKILL)
        os.waitpid(child, 0)
        self.fail("inherited code lease blocked on a host lock")

    def test_gc_retires_code_lease_after_library_close(self):
        library = self.open_expanders()
        lease = library.lease_code_on(0)
        library.close()
        self.assertTrue(library.shutdown_clean)
        del lease
        gc.collect()
        self.assertEqual(len(images()) - self.initial_images, 0 if self.release else 4)

if __name__ == "__main__":
    unittest.main()
