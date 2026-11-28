#!/usr/bin/env python3
"""Bounded real CLI processes, synthetic fixtures and SQLite; no network."""
import os
import pathlib
import signal
import subprocess
import tempfile
import unittest

DEFAULT = os.environ['SESSION_DEFAULT_BIN']
REPLAY = os.environ['SESSION_REPLAY_BIN']
REFERENCE = os.environ['SESSION_REFERENCE_BIN']
ADMIN = os.environ['SESSION_ADMIN_BIN']
NS = 'synthetic-atomic-admission-v1'

def run(binary, args=(), data=b'', code=0):
    p = subprocess.run([binary, *map(str,args)], input=data, capture_output=True, timeout=10)
    if p.returncode != code:
        raise AssertionError((p.returncode, code, p.stdout, p.stderr))
    return p

class SessionProcesses(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = pathlib.Path(self.tmp.name)

    def ledger(self, name):
        root = self.root / name
        root.mkdir(mode=0o700)
        return root

    def test_commands_history_clear_and_no_memory_fallback(self):
        p = run(DEFAULT, ['--session'], b'/help\nquestion\n/status\n/clear\n/status\nwhat is my private note?\nremember owner x\n/cancel\n/quit\n')
        text = p.stdout.decode()
        for expected in ['[mock completed]\nMock response: question', 'retained_pairs=1 retained_bytes=31',
                         'retained_pairs=0 retained_bytes=0', 'accounting=not-applicable',
                         'My local memory is unavailable.', 'Use atomos-memory', 'Unknown command', 'mock_calls=1']:
            self.assertIn(expected, text)
        self.assertNotIn('Mock response: what', text)
        self.assertNotIn('Mock response: remember', text)
        self.assertFalse(any(self.root.iterdir()))

    def test_input_utf8_limits_crlf_eof_and_no_oversized_tail(self):
        for data in [b'', b' \r\n', b'q', b'q\r\n', b'x'*1024+b'\n', ('é'*512).encode()+b'\n']:
            run(DEFAULT, ['--session'], data)
        for data in [b'x'*1025+b'\n/quit\n', b' '*1025+b'\n', b'q\r', b'q\t\n', b'\xff\n', b'\x1b[2J\n', '\u0085\n'.encode()]:
            p = run(DEFAULT, ['--session'], data, code=1)
            self.assertNotIn(b'Mock response:', p.stdout)
            self.assertNotIn(data, p.stderr)

    def test_launch_validation_precedes_all_data_access(self):
        missing = self.root / 'missing'
        cases = [ ['--session','--session'], ['--session','--single-turn'],
                  ['--session','--voice-events'], ['--session','--aec-verified'],
                  ['--session','--ledger',missing], ['--session','--init-ledger'],
                  ['--session','--replay','text','--ledger',missing],
                  ['--session','--replay','text','--ledger','--init-ledger'],
                  ['--memory',missing,'demo','--session','--replay','text','--ledger',missing] ]
        for args in cases:
            run(DEFAULT, args, code=2)
        self.assertFalse(missing.exists())
        run(REPLAY, ['--session','--replay','https://invalid','--ledger',missing], code=2)
        run(REPLAY, ['--session','--replay','text','--ledger',missing], code=1)
        self.assertFalse(missing.exists())

    def test_replay_profiles_refusal_and_clean_reopen(self):
        for name in ['text','auxiliary','reasoning','refusal']:
            root = self.ledger(name)
            args = ['--session','--replay',name,'--ledger',root]
            p = run(REPLAY, [*args,'--init-ledger'], b'question\n/status\n/quit\n')
            text = p.stdout.decode()
            self.assertIn('charged=47 background=0 pending=0 halted=false',text)
            self.assertIn('synthetic_dispatches=1',text)
            if name == 'refusal':
                self.assertIn('[synthetic refused]',text)
                self.assertIn('retained_pairs=0',text)
            else:
                self.assertIn('[synthetic completed]\nCafé 🚀\nReady.',text)
                self.assertIn('retained_pairs=1 retained_bytes=25',text)
            for secret in ['Fixture refusal','synthetic_final','resp_synthetic','encrypted_content']:
                self.assertNotIn(secret,text)
            p = run(REPLAY,args,b'/status\nquestion\n/status\n/quit\n')
            self.assertIn(b'retained_pairs=0',p.stdout)
            self.assertIn(b'charged=94',p.stdout)
            self.assertIn(b'next_id=3',p.stdout)
            before = (root/NS/'admission.log').read_bytes()
            run(REPLAY,[*args,'--init-ledger'],code=1)
            self.assertEqual(before,(root/NS/'admission.log').read_bytes())

    def test_failure_outcomes_remain_distinct_and_local_commands_survive(self):
        for name,charge,halt,exit_code in [('failed-known',47,'false',0),('failed-unknown',200,'false',1),('rate-limit',200,'true',1),('malformed',200,'false',1)]:
            root=self.ledger(name)
            args=['--session','--replay',name,'--ledger',root]
            p=run(REPLAY,[*args,'--init-ledger'],b'question\n/status\n/help\n/clear\nwhat is your name?\n/quit\n',code=exit_code)
            text=p.stdout.decode()
            self.assertIn(f'charged={charge}',text)
            self.assertIn(f'halted={halt}',text)
            self.assertIn('retained_pairs=0',text)
            self.assertIn('I am AtomOS.',text)
            self.assertNotIn('[synthetic completed]',text)
            self.assertNotIn('server_error',text)
            if name != 'failed-known':
                p=run(REPLAY,args,b'question\n/status\n/quit\n',code=1)
                self.assertIn(b'category=suspended',p.stdout)
                self.assertIn(b'synthetic_dispatches=0',p.stdout)
                self.assertIn(b'next_id=2',p.stdout)

    def test_locked_corrupt_and_wrong_configuration_never_fall_back(self):
        for kind in ['lock','corrupt','header','permissions']:
            root=self.ledger(kind)
            args=['--session','--replay','text','--ledger',root]
            run(REPLAY,[*args,'--init-ledger'],b'/quit\n')
            journal=root/NS/'admission.log'
            if kind=='lock': (root/NS/'writer.lock').touch(mode=0o600)
            elif kind=='corrupt': journal.write_bytes(journal.read_bytes()+b'A torn')
            elif kind=='header': journal.write_bytes(journal.read_bytes().replace(b'AA1 1 7',b'AA1 1 8',1))
            else: journal.chmod(0o644)
            original=journal.read_bytes()
            p=run(REPLAY,args,b'question\n',code=1)
            self.assertNotIn(b'local/mock',p.stdout)
            self.assertEqual(original,journal.read_bytes())

    def test_default_sigint_leaves_stale_lock_without_cleanup(self):
        root=self.ledger('death')
        args=['--session','--replay','failed-unknown','--ledger',root]
        p=subprocess.Popen([REPLAY,*map(str,args),'--init-ledger'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        self.addCleanup(lambda: p.kill() if p.poll() is None else None)
        self.assertIn(b'synthetic-replay',p.stdout.readline())
        p.stdin.write(b'question\n');p.stdin.flush()
        self.assertIn(b'usage=unknown',p.stdout.readline())
        p.send_signal(signal.SIGINT)
        p.communicate(timeout=5)
        self.assertEqual(p.returncode,-signal.SIGINT)
        self.assertTrue((root/NS/'writer.lock').exists())
        journal=(root/NS/'admission.log').read_bytes()
        self.assertIn(b' 200\n',journal)
        run(REPLAY,args,code=1)
        self.assertTrue((root/NS/'writer.lock').exists())
        self.assertEqual(journal,(root/NS/'admission.log').read_bytes())

    def test_memory_requeries_corrections_and_never_enters_working_history(self):
        store=self.root/'memory'
        def admin(*args,value=b''):
            return run(ADMIN,[store,'demo',*args],value).stdout
        admin('init','Aster')
        admin('remember','owner','favorite_color','text',value=b'indigo')
        p=subprocess.Popen([DEFAULT,'--memory',str(store),'demo','--session'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
        self.addCleanup(lambda:p.kill() if p.poll() is None else None)
        p.stdout.readline()
        def ask(question,lines=2):
            p.stdin.write(question+'\n');p.stdin.flush()
            return ''.join(p.stdout.readline() for _ in range(lines))
        self.assertEqual(ask('what is my favorite color?'),'[local]\nI have favorite color recorded as indigo.\n')
        revision=admin('recall','owner','favorite_color').decode().split()[0].split('=')[1]
        admin('correct','owner','favorite_color',revision,'text',value=b'copper')
        self.assertIn('copper',ask('and favorite color?'))
        self.assertIn('retained_pairs=0 retained_bytes=0',ask('/status'))
        ask('/clear',1)
        self.assertIn('Please specify',ask('and favorite color?'))
        self.assertIn('Use atomos-memory',ask('forget owner favorite_color'))
        admin('forget','owner','favorite_color')
        self.assertIn('do not have that fact',ask('what is my favorite color?'))
        p.stdin.write('/quit\n');p.stdin.flush();p.communicate(timeout=5)
        self.assertEqual(p.returncode,0)

    def test_legacy_output_exit_and_memory_match_frozen_reference(self):
        store=self.root/'legacy'
        run(ADMIN,[store,'demo','init','Aster'])
        for args,data in [([],b'what is your name?\nquestion\n/quit\n'),(['--single-turn'],b'question\n'),(['--voice-events'],b'ARM\n'),(['--voice-events','--aec-verified'],b'ARM\n'),(['--memory',store,'demo'],b'what is your name?\nrecall owner favorite_color\n/quit\n')]:
            old=run(REFERENCE,args,data)
            new=run(DEFAULT,args,data)
            self.assertEqual((old.stdout,old.stderr,old.returncode),(new.stdout,new.stderr,new.returncode))

if __name__=='__main__':
    unittest.main(verbosity=2)
