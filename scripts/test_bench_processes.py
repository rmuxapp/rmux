import unittest
from bench_processes import tree, retained_cpu, seconds


def row(pid, parent, rss, cpu=0, started="first"):
    return dict(pid=pid, parent_pid=parent, rss_kib=rss, cpu_seconds=cpu, started=started)


class ProcessAccounting(unittest.TestCase):
    def test_nested_tree_excludes_unrelated_processes(self):
        result = tree(10, [row(30, 20, 3), row(20, 10, 2), row(99, 1, 100), row(10, 1, 10)])
        self.assertEqual([entry["pid"] for entry in result["processes"]], [10, 20, 30])
        self.assertEqual(result["rss_sum_kib"], 15)
        self.assertEqual(result["child_rss_sum_kib"], 5)
        with self.assertRaises(ProcessLookupError):
            tree(11, [])

    def test_cpu_excludes_reused_pids_and_reports_missing_processes(self):
        before = tree(10, [row(10, 1, 1, 2), row(20, 10, 1, 8), row(30, 10, 1, 4)])
        after = tree(10, [row(10, 1, 1, 3), row(20, 10, 1, 100, "replacement")])
        self.assertEqual(retained_cpu(before, after), dict(cpu_seconds=1, retained_processes=1,
                         disappeared_processes=2, appeared_processes=1))
        self.assertEqual(seconds("1-02:03:04.50"), 93784.5)


if __name__ == "__main__":
    unittest.main()
