"""Count actual admission and settlement journal syscalls in isolated runs."""
import re
import sys

mode, path = sys.argv[1:]
phase = None
counts = {p: {"write": 0, "fsync": 0} for p in ("admission", "settlement")}
starts = reads = ends = 0
for line in open(path, encoding="utf-8"):
    if "TRACE_START" in line:
        assert phase is None
        phase = "admission"
        starts += 1
    elif "TRACE_READ" in line:
        assert phase == "admission"
        phase = "settlement"
        reads += 1
    elif "TRACE_END" in line:
        assert phase == "settlement"
        phase = None
        ends += 1
    elif phase and re.search(r"/admission\.log>", line):
        assert "= -1" not in line, line
        for call in ("write", "fsync"):
            if line.startswith(call + "("):
                counts[phase][call] += 1
assert starts == reads == ends == 3 and phase is None, (mode, starts, reads, ends)
for phase, values in counts.items():
    assert values["fsync"] == starts, (mode, phase, values)
    assert values["write"] >= starts, (mode, phase, values)
    print(f"mode={mode} phase={phase} requests={starts} journal_write_syscalls_per_request={values['write']/starts:g} journal_fsyncs_per_request={values['fsync']/starts:g}")
