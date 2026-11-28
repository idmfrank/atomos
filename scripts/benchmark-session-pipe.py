"""New CLI through actual stdin/stdout pipes; not physical terminal latency."""
import pathlib
import subprocess
import sys
import time

binary, root, raw_samples = sys.argv[1:]
samples = int(raw_samples)
assert 1 <= samples <= 1000
root=pathlib.Path(root)
root.mkdir(mode=0o700)
p=subprocess.Popen([binary,'--session','--replay','text','--ledger',str(root),'--init-ledger'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
try:
    assert b'synthetic-replay' in p.stdout.readline()
    values=[]
    for _ in range(samples):
        start=time.perf_counter_ns()
        p.stdin.write(b'question\n');p.stdin.flush()
        assert p.stdout.readline()==b'[synthetic completed]\n'
        assert p.stdout.readline()=='Café 🚀\n'.encode()
        assert p.stdout.readline()==b'Ready.\n'
        values.append((time.perf_counter_ns()-start)/1e6)
    p.stdin.write(b'/status\n/quit\n');p.stdin.flush()
    out,err=p.communicate(timeout=10)
    assert p.returncode==0 and not err
    assert f'charged={samples*47}'.encode() in out
    assert b'background=0' in out and f'retained_pairs={min(samples,4)}'.encode() in out
    values.sort()
    def percentile(q): return values[max(0,(samples*q+99)//100-1)]
    print(f'metric=cli_pipe_end_to_end samples={samples} p50_ms={percentile(50):.6f} p95_ms={percentile(95):.6f} p99_ms={percentile(99):.6f} errors=0 charged={samples*47} real_calls=0')
finally:
    if p.poll() is None: p.kill();p.communicate()
