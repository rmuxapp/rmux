"""Read-only macOS process-tree accounting; never captures command arguments."""
import subprocess


def seconds(value):
    days, clock = value.split("-", 1) if "-" in value else ("0", value)
    result = 0.0
    for part in clock.split(":"):
        result = result * 60 + float(part)
    return int(days) * 86400 + result


def process_table():
    output = subprocess.run(["/bin/ps", "-axo", "pid=,ppid=,rss=,time=,lstart="],
                            check=True, capture_output=True, text=True, timeout=10).stdout
    rows = []
    for line in output.splitlines():
        pid, parent, rss, cpu, started = line.split(None, 4)
        rows.append(dict(pid=int(pid), parent_pid=int(parent), rss_kib=int(rss),
                         cpu_seconds=seconds(cpu), started=started.strip()))
    return rows


def tree(root, rows=None):
    rows = process_table() if rows is None else rows
    selected = {root}
    while True:
        children = {row["pid"] for row in rows if row["parent_pid"] in selected}
        if children <= selected:
            break
        selected |= children
    members = [row for row in rows if row["pid"] in selected]
    if not any(row["pid"] == root for row in members):
        raise ProcessLookupError(root)
    members.sort(key=lambda row: row["pid"])
    return dict(root_pid=root, processes=members,
                rss_sum_kib=sum(row["rss_kib"] for row in members),
                child_rss_sum_kib=sum(row["rss_kib"] for row in members if row["pid"] != root))


def retained_cpu(before, after):
    """Deltas only for identities alive in both samples; exited CPU is unknown."""
    previous = {(row["pid"], row["started"]): row for row in before["processes"]}
    current = {(row["pid"], row["started"]): row for row in after["processes"]}
    retained = previous.keys() & current.keys()
    return dict(cpu_seconds=sum(max(0, current[key]["cpu_seconds"] - previous[key]["cpu_seconds"]) for key in retained),
                retained_processes=len(retained), disappeared_processes=len(previous.keys() - current.keys()),
                appeared_processes=len(current.keys() - previous.keys()))


def survivors(before):
    live = {(row["pid"], row["started"]) for row in process_table()}
    return [row["pid"] for row in before["processes"]
            if row["pid"] != before["root_pid"] and (row["pid"], row["started"]) in live]
