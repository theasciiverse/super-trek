#!/usr/bin/env python3
"""Differential fuzzing of Super Trek against SST2K.

Generates random games (seed, game settings and a random stream of plausible
commands), replays each through SST2K's Python `sst` and through the port,
and reports every game whose output differs. Failing games are kept as
replay logs in the output folder, ready to become recorded tests.

    tools/fuzz.py [--games N] [--jobs J] [--start SEED] [--bin PATH] [--out DIR]
"""
import argparse
import concurrent.futures
import os
import random
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
SST = os.path.join(ROOT, "..", "..", "references", "super-star-trek", "sst")


def coord(r):
    return "%d %d" % (r.randint(1, 10), r.randint(1, 10))


def quad(r):
    return "%d %d" % (r.randint(1, 8), r.randint(1, 8))


def alpha(r):
    return "%s%d" % ("abcdefghij"[r.randint(0, 9)], r.randint(1, 10))


def command(r):
    """One random command line (sometimes with follow-up answer lines)."""
    c = r.random()
    choices = [
        lambda: "srscan",
        lambda: "lrscan",
        lambda: "status",
        lambda: "chart",
        lambda: "damages",
        lambda: "report",
        lambda: "score",
        lambda: "planets",
        lambda: "sensors",
        lambda: "request " + r.choice(["date", "condition", "position", "lsupport", "warpfactor",
                                        "energy", "torpedoes", "shields", "klingons", "time", "bases", "x"]),
        lambda: "move m %d %d" % (r.randint(-6, 6), r.randint(-6, 6)),
        lambda: "move m %.1f %.1f" % (r.uniform(-3, 3), r.uniform(-3, 3)),
        lambda: "move a " + r.choice([coord(r), quad(r) + " " + coord(r), alpha(r), alpha(r) + " " + alpha(r)]),
        lambda: "move",
        lambda: "impulse m %d %d" % (r.randint(-2, 2), r.randint(-2, 2)),
        lambda: "impulse a " + coord(r),
        lambda: "warp %d" % r.randint(1, 10),
        lambda: "warp %.1f" % r.uniform(0.5, 10.5),
        lambda: "phasers %d" % r.randint(1, 1500),
        lambda: "phasers a %d" % r.randint(1, 1500),
        lambda: "phasers m " + " ".join(str(r.randint(0, 600)) for _ in range(r.randint(1, 4))),
        lambda: "phasers",
        lambda: "torpedo %d " % r.randint(1, 3) + " ".join(coord(r) for _ in range(r.randint(1, 3))),
        lambda: "torpedo 1 " + alpha(r),
        lambda: "photons " + coord(r),
        lambda: "shields " + r.choice(["up", "down", "transfer %d" % r.randint(-500, 2000), "u", "d", "t"]),
        lambda: "shields",
        lambda: "dock",
        lambda: "rest %.1f" % r.uniform(0.1, 6),
        lambda: "rest",
        lambda: "orbit",
        lambda: "transport",
        lambda: "transport y",
        lambda: "mine",
        lambda: "crystals",
        lambda: "shuttle",
        lambda: "shuttle y",
        lambda: "probe " + r.choice(["y", "n", ""]) + " " + r.choice(["a " + quad(r), "m %d %d" % (r.randint(-5, 5), r.randint(-5, 5)), ""]),
        lambda: "computer " + r.choice([coord(r), quad(r) + " " + coord(r), alpha(r)]) + " %d" % r.randint(1, 10),
        lambda: "computer",
        lambda: "capture",
        lambda: "cloak " + r.choice(["on", "off", ""]),
        lambda: "deathray",
        lambda: "mayday",
        lambda: "abandon",
        lambda: "destruct",
        lambda: "options",
        lambda: "options " + r.choice(["set", "clear"]) + " " + r.choice(["ALPHAMERIC", "SHOWME", "DOTFILL", "AUTOSCAN", "BASE", "THOLIAN", "FOO"]),
        lambda: "help " + r.choice(["phasers", "move", "abbrev", "xyzzy", "srscan", "options"]),
        lambda: "commands",
        lambda: r.choice(["y", "n", "yes", "no", "", "1", "a", "frob"]),
    ]
    weights = [3, 3, 2, 2, 2, 1, 1, 1, 1, 1, 6, 4, 4, 1, 2, 1, 2, 1, 4, 3, 2, 1, 4, 1, 1, 3, 1, 2, 3]
    weights += [1] * (len(choices) - len(weights) - 1) + [3]
    f = r.choices(choices, weights)[0]
    return f()


def make_game(seed):
    r = random.Random(seed)
    gametype = r.choice(["regular"] * 6 + ["tournament %d" % r.randint(1, 999)])
    length = r.choice(["short", "medium", "long", "sandbox"])
    skill = r.choice(["novice", "fair", "good", "expert", "emeritus"])
    year = r.choice(["1973", "1976", "1979", "1980", "1982", "1997", "2004", "2006", "2013", "2023", "2026"])
    lines = ["# seed %d" % r.randint(0, 2**31), "# arguments ", "# fuzz game %d" % seed, "#",
             gametype, length, skill, year, r.choice(["secret", "pw"])]
    for _ in range(r.randint(10, 120)):
        lines.append(command(r))
        if r.random() < 0.3:
            lines.append(r.choice(["y", "n"]))
    lines.append("quit")
    return "\n".join(lines) + "\n"


def run(argv, cwd, env):
    p = subprocess.run(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL, capture_output=True, timeout=60)
    return p.stdout + p.stderr, p.returncode


def one(seed, binary, out):
    log = make_game(seed)
    with tempfile.TemporaryDirectory(prefix="trekfuzz-") as d:
        # SST2K reads its help from ./sst.doc; give it the port's copy
        import shutil
        shutil.copy(os.path.join(ROOT, "assets", "sst.doc"), d)
        path = os.path.join(d, "g.log")
        with open(path, "w") as f:
            f.write(log)
        env = dict(os.environ, TMPDIR=d, XDG_CACHE_HOME=d, XDG_STATE_HOME=d, SUPER_TREK_FAST="1")
        try:
            py, pyrc = run([sys.executable, SST, "-r", path], d, env)
        except subprocess.TimeoutExpired:
            return seed, "python-timeout"
        if b"Traceback" in py:
            return seed, "python-crash"
        try:
            rs, rsrc = run([binary, "-r", path], d, env)
        except subprocess.TimeoutExpired:
            rs, rsrc = b"TIMEOUT", -1
        if py == rs and pyrc == rsrc:
            return seed, "ok"
        os.makedirs(out, exist_ok=True)
        with open(os.path.join(out, "fuzz%d.log" % seed), "w") as f:
            f.write(log)
        with open(os.path.join(out, "fuzz%d.py.out" % seed), "wb") as f:
            f.write(py)
        with open(os.path.join(out, "fuzz%d.rs.out" % seed), "wb") as f:
            f.write(rs)
        return seed, "DIFF"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--games", type=int, default=200)
    ap.add_argument("--jobs", type=int, default=8)
    ap.add_argument("--start", type=int, default=1)
    ap.add_argument("--bin", default=os.path.join(ROOT, "target", "release", "super-trek"))
    ap.add_argument("--out", default=os.path.join(tempfile.gettempdir(), "super-trek-fuzz"))
    a = ap.parse_args()
    counts = {}
    diffs, crashes = [], []
    with concurrent.futures.ThreadPoolExecutor(a.jobs) as ex:
        for seed, res in ex.map(lambda s: one(s, a.bin, a.out), range(a.start, a.start + a.games)):
            counts[res] = counts.get(res, 0) + 1
            if res == "DIFF":
                diffs.append(seed)
            elif res.startswith("python"):
                crashes.append(seed)
    print(counts)
    print("differing:", diffs)
    print("python crashed or hung (skipped):", crashes)
    print("logs in", a.out)


if __name__ == "__main__":
    main()
