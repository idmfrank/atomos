"""Count actual journal write/fsync syscalls between synthetic admission markers."""
import re
import sys

mode, path = sys.argv[1:]
active = False
writes = syncs = requests = 0
for line in open(path, encoding="utf-8"):
    if 'TRACE_START' in line:
        active = True
        requests += 1
    elif 'TRACE_READ' in line:
        active = False
    elif active and re.search(r'/(policy|usage|admission)\.log>', line):
        assert '= -1' not in line, line
        if line.startswith('write('):
            writes += 1
        elif line.startswith('fsync('):
            syncs += 1
assert requests == 3, (mode, requests)
assert syncs == requests * (2 if mode == 'reference' else 1), (mode, syncs)
print(f'mode={mode} traced_admissions={requests} journal_write_syscalls_per_admission={writes/requests:g} journal_fsyncs_per_admission={syncs/requests:g}')
