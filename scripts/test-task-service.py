#!/usr/bin/env python3
"""Synthetic process lifecycle tests; no target configuration or network."""
import os
from pathlib import Path
import select
import signal
import sqlite3
import subprocess
import tempfile
import time
import unittest

ROOT=Path(__file__).resolve().parents[1]
BIN=Path(os.environ.get('TASKD_BIN',ROOT/'target/debug/atomos-taskd'))

class Service(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.root=Path(self.tmp.name);self.store=self.root/'store'

    def start(self,delay=0,init=True,daemon=False,store=None):
        args=[str(BIN),'--store',str(store or self.store),'--owner','fixture']
        if init:args+=['--init-bytes',str(4*1024*1024)]
        if delay:args+=['--fixture-delay-ms',str(delay)]
        if daemon:args+=['--daemon']
        p=subprocess.Popen(args,stdin=subprocess.DEVNULL if daemon else subprocess.PIPE,
                           stdout=subprocess.PIPE,stderr=subprocess.PIPE,bufsize=0)
        self.addCleanup(self.cleanup,p)
        self.assertEqual(self.line(p),'READY synthetic=1');return p

    def cleanup(self,p):
        if p.poll() is None:
            p.terminate()
            try:p.wait(timeout=3)
            except subprocess.TimeoutExpired:p.kill();p.wait(timeout=3)
        for stream in (p.stdin,p.stdout,p.stderr):
            if stream:stream.close()

    def line(self,p):
        self.assertTrue(select.select([p.stdout],[],[],5)[0],'response timeout')
        data=p.stdout.readline()
        self.assertTrue(data,'unexpected service exit')
        return data.decode('ascii').rstrip('\n')

    def send(self,p,line):
        p.stdin.write(line.encode('ascii')+b'\n');p.stdin.flush();return self.line(p)

    def submit(self,p,key):
        out=self.send(p,'SUBMIT '+key+' '+b'synthetic private goal'.hex())
        self.assertTrue(out.startswith('TASK '),out);return int(out.split()[1])

    def wait(self,p,id,needle):
        end=time.monotonic()+5
        while True:
            out=self.send(p,f'STATUS {id}')
            if needle in out:return out
            self.assertLess(time.monotonic(),end,out);time.sleep(.005)

    def stop(self,p):
        p.stdin.write(b'STOP\n');p.stdin.flush()
        self.assertEqual(p.wait(timeout=5),0);self.assertEqual(p.stdout.read(),b'')

    def state(self,id):
        with sqlite3.connect(self.store.joinpath('tasks.db').as_uri()+'?mode=ro',uri=True) as c:
            return c.execute('SELECT state FROM task WHERE id=?',(id,)).fetchone()[0]

    def counter(self):
        with sqlite3.connect(self.store.joinpath('tasks.db').as_uri()+'?mode=ro',uri=True) as c:
            row=c.execute("SELECT value FROM counter WHERE item='worker-fixture'").fetchone()
            return row[0] if row else 0

    def test_stdio_completion_idempotency_and_redacted_responses(self):
        p=self.start();id=self.submit(p,'one');out=self.wait(p,id,'Completed')
        self.assertIn('dispatches=1',out);self.assertNotIn('private goal',out)
        self.assertEqual(self.submit(p,'one'),id)
        self.assertIn('Conflict',self.send(p,'SUBMIT one '+b'other'.hex()))
        self.stop(p);self.assertEqual(self.counter(),1)

    def test_thirty_cancellations_ack_and_worker_stop_are_bounded(self):
        p=self.start(delay=30_000);samples=[]
        for n in range(30):
            id=self.submit(p,f'cancel-{n}');self.wait(p,id,'dispatches=1')
            start=time.monotonic();self.assertIn('Cancelled',self.send(p,f'CANCEL {id}'))
            samples.append(time.monotonic()-start)
        self.assertLess(sorted(samples)[28],1)
        ordered=sorted(samples)
        print(f'worker_cancel samples=30 p50_ms={ordered[15]*1000:.6f} p95_ms={ordered[28]*1000:.6f} p99_ms={ordered[29]*1000:.6f} effects=0',flush=True)
        self.stop(p);self.assertEqual(self.counter(),0)

    def test_term_and_int_interrupt_and_restart_without_reexecution(self):
        for sig in (signal.SIGTERM,signal.SIGINT):
            with self.subTest(signal=sig):
                self.store=self.root/f'case-{sig}'
                p=self.start(delay=30_000);id=self.submit(p,'active');self.wait(p,id,'dispatches=1')
                start=time.monotonic();p.send_signal(sig)
                self.assertEqual(p.wait(timeout=5),0);self.assertLess(time.monotonic()-start,5)
                self.assertEqual(p.stdout.read(),b'');self.assertEqual(self.state(id),'interrupted')
                p=self.start(init=False);self.assertIn('Interrupted',self.send(p,f'STATUS {id}'))
                self.assertIn('Ambiguous',self.send(p,f'RESUME {id}'))
                self.assertIn('Cancelled',self.send(p,f'CANCEL {id}'))
                next_id=self.submit(p,'next');self.wait(p,next_id,'Completed');self.stop(p)
                self.assertEqual(self.counter(),1)

    def test_forced_death_preserves_queue_and_ambiguous_intent(self):
        p=self.start(delay=30_000);first=self.submit(p,'first');self.wait(p,first,'dispatches=1')
        second=self.submit(p,'second');p.kill();p.wait(timeout=5)
        p=self.start(init=False);self.wait(p,second,'Completed')
        self.assertEqual(self.submit(p,'second'),second)
        self.assertIn('Ambiguous',self.send(p,f'RESUME {first}'))
        self.assertIn('Cancelled',self.send(p,f'CANCEL {first}'));self.stop(p)
        self.assertEqual(self.counter(),1)

    def test_invalid_args_input_caps_and_unknown_commands_do_not_echo(self):
        for extra in (['--unknown'],['--daemon','--daemon'],['--fixture-delay-ms','0']):
            p=subprocess.run([str(BIN),'--store',str(self.store),'--owner','fixture','--init-bytes','4194304',*extra],capture_output=True,timeout=5)
            self.assertNotEqual(p.returncode,0);self.assertEqual(p.stdout,b'');self.assertFalse(self.store.exists())
        p=self.start()
        for text in ['UNKNOWN private-fixture-text','SUBMIT key zz','SUBMIT key '+('78'*8193),'EVENTS 999 1']:
            out=self.send(p,text);self.assertTrue(out.startswith('ERROR '));self.assertNotIn('private-fixture-text',out)
        p.stdin.write(b'x'*16641+b'\n');p.stdin.flush()
        self.assertEqual(self.line(p),'ERROR Invalid');self.assertEqual(p.wait(timeout=5),0);self.assertEqual(p.stdout.read(),b'')

    def test_competing_instance_and_missing_store_fail_without_reset(self):
        p=self.start()
        for store in [self.store,self.root/'missing']:
            q=subprocess.run([str(BIN),'--store',str(store),'--owner','fixture'],capture_output=True,timeout=5)
            self.assertNotEqual(q.returncode,0);self.assertEqual(q.stdout,b'')
        self.assertFalse((self.root/'missing').exists());self.stop(p)

    def test_eof_quiesces_stdio_but_daemon_ignores_stdin_eof(self):
        p=self.start(delay=30_000);id=self.submit(p,'eof');self.wait(p,id,'dispatches=1')
        p.stdin.close();self.assertEqual(p.wait(timeout=5),0);self.assertEqual(p.stdout.read(),b'')
        self.assertEqual(self.state(id),'interrupted')
        p=self.start(init=False,daemon=True);time.sleep(.2);self.assertIsNone(p.poll())
        p.terminate();self.assertEqual(p.wait(timeout=5),0);self.assertEqual(p.stdout.read(),b'')
        self.assertEqual(self.counter(),0)

    def test_unread_stdout_cannot_block_signal_shutdown(self):
        p=self.start();id=self.submit(p,'backpressure');self.wait(p,id,'Completed')
        import threading
        def flood():
            try:
                for _ in range(2000):p.stdin.write(f'STATUS {id}\n'.encode('ascii'))
            except (OSError,ValueError):pass
        producer=threading.Thread(target=flood,daemon=True);producer.start()
        time.sleep(.2)  # Intentionally leave stdout unread while it fills.
        p.terminate();self.assertEqual(p.wait(timeout=5),0)
        producer.join(timeout=1)

    def test_supervisor_template_parses_without_installing_a_service(self):
        source=(ROOT/'deploy/atomos-taskd.service.in').read_text(encoding='utf-8')
        store=self.root/'unit-store';store.mkdir(mode=0o700)
        rendered=source.replace('@BINARY@','/usr/bin/true').replace('@STORE@',str(store)).replace('@OWNER@','fixture')
        self.assertNotIn('@',rendered)
        unit=self.root/'atomos-fixture.service';unit.write_text(rendered,encoding='utf-8')
        result=subprocess.run(['systemd-analyze','--man=no','verify',str(unit)],capture_output=True,text=True,timeout=10)
        self.assertEqual(result.returncode,0,result.stderr)
        # Parser verification with a dummy executable is not service installation,
        # boot testing, user-namespace support or real isolation qualification.

    def test_redirected_output_preserves_readiness_and_response_order(self):
        output=self.root/'output.log'
        with output.open('wb') as stream:
            p=subprocess.Popen([str(BIN),'--store',str(self.store),'--owner','fixture','--init-bytes','4194304'],
                               stdin=subprocess.PIPE,stdout=stream,stderr=subprocess.PIPE,bufsize=0)
            self.addCleanup(self.cleanup,p)
            end=time.monotonic()+5
            while b'READY synthetic=1\n' not in output.read_bytes():
                self.assertLess(time.monotonic(),end);time.sleep(.005)
            p.stdin.write(b'SUBMIT redirect 66697874757265\n');p.stdin.flush()
            while b'TASK 1 ' not in output.read_bytes():
                self.assertLess(time.monotonic(),end);time.sleep(.005)
            p.stdin.close();self.assertEqual(p.wait(timeout=5),0)
        data=output.read_bytes()
        self.assertTrue(data.startswith(b'READY synthetic=1\nTASK 1 '))

if __name__=='__main__':unittest.main(verbosity=2)
