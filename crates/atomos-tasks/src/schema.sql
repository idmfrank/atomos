CREATE TABLE configuration (
 singleton INTEGER PRIMARY KEY CHECK(singleton=1),
 owner TEXT NOT NULL,
 policy TEXT NOT NULL,
 page_cap INTEGER NOT NULL CHECK(page_cap BETWEEN 16 AND 65536),
 last_time INTEGER NOT NULL CHECK(last_time>=0)
) STRICT;
CREATE TABLE task (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 submission_key TEXT UNIQUE NOT NULL,
 goal TEXT NOT NULL CHECK(length(CAST(goal AS BLOB)) BETWEEN 1 AND 8192),
 state TEXT NOT NULL CHECK(state IN ('queued','running','waiting_approval','cancelling','interrupted','completed','failed','cancelled')),
 dispatches INTEGER NOT NULL DEFAULT 0 CHECK(dispatches BETWEEN 0 AND 32),
 active_ms INTEGER NOT NULL DEFAULT 0 CHECK(active_ms>=0),
 failure TEXT CHECK(failure IN ('task_time','dispatch_limit','action_deadline','fixture_mismatch')),
 running_since INTEGER CHECK(running_since>=0)
) STRICT;
CREATE UNIQUE INDEX one_active ON task((1)) WHERE state IN ('running','cancelling');
CREATE TABLE action (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 task_id INTEGER NOT NULL REFERENCES task(id),
 kind TEXT NOT NULL CHECK(kind IN ('add','assert')),
 item TEXT NOT NULL,
 argument INTEGER NOT NULL,
 expected INTEGER NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('prepared','interrupted','verified','failed','cancelled')),
 intent_time INTEGER NOT NULL CHECK(intent_time>=0)
) STRICT;
CREATE UNIQUE INDEX one_unresolved ON action(task_id) WHERE state IN ('prepared','interrupted');
CREATE TABLE counter (item TEXT PRIMARY KEY, value INTEGER NOT NULL) STRICT;
CREATE TABLE effect (
 action_id INTEGER PRIMARY KEY REFERENCES action(id),
 observed INTEGER NOT NULL,
 passed INTEGER NOT NULL CHECK(passed IN (0,1))
) STRICT;
CREATE TABLE event (
 cursor INTEGER PRIMARY KEY AUTOINCREMENT,
 task_id INTEGER NOT NULL REFERENCES task(id),
 state TEXT NOT NULL,
 time INTEGER NOT NULL CHECK(time>=0)
) STRICT;
