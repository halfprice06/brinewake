"""8x8 stat marks for the hover card's number rows: the resources (after
the top bar's crate, glass and head) and the machine's numbers.  Drawn in
the top-left corner of a 24x24 frame; the sheet crops them to 8x8."""

from icons import icon, Icon

SMALL = {}


def mark(key):
    def wrap(fn):
        SMALL[key] = 8
        return icon(key)(fn)

    return wrap


def rows(r):
    ic = Icon()
    ic.rows(0, 0, r)
    return ic


@mark("tip_salvage")
def salvage():
    return rows([
        "..kkkk..",
        ".kPPPpk.",
        "kPPPppok",
        "kOOOooOk",
        "kOkkOoOk",
        "kOOOooOk",
        "kOOOoook",
        "kkkkkkkk",
    ])


@mark("tip_pressure")
def pressure():
    return rows([
        "kkkkkkkk",
        "khhhgggk",
        ".knBbbk.",
        ".knBbbk.",
        ".knBbbk.",
        ".knBbbk.",
        "kgffeeek",
        "kkkkkkkk",
    ])


@mark("tip_crew")
def crew():
    return rows([
        ".kkkkkk.",
        "kWCCCCck",
        "kCBnBBck",
        "kCBBBBck",
        "kCCCCcck",
        ".kkkkkk.",
        "kffggffk",
        "kkkkkkkk",
    ])


@mark("tip_time")
def time():
    return rows([
        "..kkkk..",
        ".kCWWCk.",
        "kCWkWCck",
        "kCWkkCck",
        "kCCCCCck",
        "kcCCCcck",
        ".kcccck.",
        "..kkkk..",
    ])


@mark("tip_damage")
def damage():
    return rows([
        "k..kk..k",
        ".kkyykk.",
        ".kyWWyk.",
        "kyWWWyyk",
        "kyyWyyAk",
        ".kyyyAk.",
        ".kkAAkk.",
        "k..kk..k",
    ])


@mark("tip_reach")
def reach():
    return rows([
        "...kk...",
        "..kgfk..",
        ".kgk.fk.",
        "kgkPPkfk",
        "kgkPPkfk",
        ".kgk.fk.",
        "..kgfk..",
        "...kk...",
    ])


@mark("tip_hull")
def hull():
    return rows([
        "kkkkkkkk",
        "khhhhhgk",
        "khgggggk",
        "khgdggfk",
        "khgggffk",
        ".kgggfk.",
        "..kgfk..",
        "...kk...",
    ])


@mark("tip_speed")
def speed():
    return rows([
        "kk..kk..",
        "kmk.kmk.",
        ".kmk.kmk",
        "..kmk.km",
        "..kJk.kJ",
        ".kJk.kJk",
        "kJk.kJk.",
        "kk..kk..",
    ])


@mark("tip_sight")
def sight():
    return rows([
        "........",
        "..kkkk..",
        ".kCCCCk.",
        "kCkBBkCk",
        "kCkBnkCk",
        ".kCCCCk.",
        "..kkkk..",
        "........",
    ])
