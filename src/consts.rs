//! Constants and tables from `sst` (109-153, 412-513, 677-699, 6816-6880,
//! 7458-7506), names unchanged.

pub const VERSION: &str = "2.10";

pub const GALSIZE: i64 = 8;
pub const NINHAB: i64 = GALSIZE * GALSIZE / 2;
pub const MAXUNINHAB: i64 = 10;
pub const QUADSIZE: i64 = 10;
pub const BASEMIN: i64 = 2;
pub const BASEMAX: i64 = GALSIZE * GALSIZE / 12;
pub const MAXKLGAME: i64 = 127;
pub const MAXKLQUAD: i64 = 9;
pub const FULLCREW: i64 = 428;
pub const FOREVER: f64 = 1e30;
pub const MAXBURST: i64 = 3;
pub const MAXCMDR: i64 = 10;
pub const DOCKFAC: f64 = 0.25;
pub const PHASEFAC: f64 = 2.0;
pub const ALGERON: f64 = 2311.0;

pub const GS: usize = GALSIZE as usize;
pub const QS: usize = QUADSIZE as usize;

// Colours (no-ops in line mode; kept for the UI)
pub const DEFAULT: i64 = -1;
pub const BLACK: i64 = 0;
pub const BLUE: i64 = 1;
pub const GREEN: i64 = 2;
pub const CYAN: i64 = 3;
pub const RED: i64 = 4;
pub const MAGENTA: i64 = 5;
pub const BROWN: i64 = 6;
pub const LIGHTGRAY: i64 = 7;
pub const DARKGRAY: i64 = 8;
pub const LIGHTBLUE: i64 = 9;
pub const LIGHTGREEN: i64 = 10;
pub const LIGHTCYAN: i64 = 11;
pub const LIGHTRED: i64 = 12;
pub const LIGHTMAGENTA: i64 = 13;
pub const YELLOW: i64 = 14;
pub const WHITE: i64 = 15;

// game options
pub const OPTION_ALL: i64 = 0xFFFF_FFFF;
pub const OPTION_TTY: i64 = 1 << 0;
pub const OPTION_CURSES: i64 = 1 << 1;
pub const OPTION_PLANETS: i64 = 1 << 2;
pub const OPTION_BSDTREK: i64 = 1 << 3;
pub const OPTION_THOLIAN: i64 = 1 << 4;
pub const OPTION_SUPERCMDR: i64 = 1 << 5;
pub const OPTION_PROBE: i64 = 1 << 6;
pub const OPTION_MVBADDY: i64 = 1 << 7;
pub const OPTION_RAMMING: i64 = 1 << 8;
pub const OPTION_ALMY: i64 = 1 << 9;
pub const OPTION_AUTOPASS: i64 = 1 << 10;
pub const OPTION_BASE: i64 = 1 << 11;
pub const OPTION_BLKHOLE: i64 = 1 << 12;
pub const OPTION_SHOWME: i64 = 1 << 13;
pub const OPTION_WORLDS: i64 = 1 << 14;
pub const OPTION_AUTOSCAN: i64 = 1 << 15;
pub const OPTION_COLOR: i64 = 1 << 16;
pub const OPTION_CAPTURE: i64 = 1 << 17;
pub const OPTION_CLOAK: i64 = 1 << 18;
pub const OPTION_DOTFILL: i64 = 1 << 19;
pub const OPTION_ALPHAMERIC: i64 = 1 << 20;
pub const OPTION_IOMODES: i64 = OPTION_CURSES | OPTION_TTY;

/// `option_names`, in the Python dict's insertion order. BSDTREK maps to
/// the PLANETS bit, as in SST2K (a known quirk, kept).
pub const OPTION_NAMES: &[(&str, i64, i64)] = &[
    ("ALL", OPTION_ALL, 0),
    ("TTY", OPTION_TTY, 0),
    ("IOMODES", OPTION_IOMODES, 0),
    ("PLANETS", OPTION_PLANETS, 1974),
    ("BSDTREK", OPTION_PLANETS, 1976),
    ("THOLIAN", OPTION_THOLIAN, 1979),
    ("SUPERCMDR", OPTION_SUPERCMDR, 1979),
    ("PROBE", OPTION_PROBE, 1980),
    ("MVBADDY", OPTION_MVBADDY, 1981),
    ("RAMMING", OPTION_RAMMING, 1982),
    ("ALMY", OPTION_ALMY, 1997),
    ("AUTOPASS", OPTION_AUTOPASS, 1998),
    ("BASE", OPTION_BASE, 2004),
    ("BLKHOLE", OPTION_BLKHOLE, 2004),
    ("SHOWME", OPTION_SHOWME, 2005),
    ("WORLDS", OPTION_WORLDS, 2006),
    ("AUTOSCAN", OPTION_AUTOSCAN, 2007),
    ("COLOR", OPTION_COLOR, 2010),
    ("CAPTURE", OPTION_CAPTURE, 2013),
    ("CLOAK", OPTION_CLOAK, 2014),
    ("DOTFILL", OPTION_DOTFILL, 2019),
    ("ALPHAMERIC", OPTION_ALPHAMERIC, 2023),
];

pub fn option_lookup(name: &str) -> Option<i64> {
    OPTION_NAMES.iter().find(|(n, _, _)| *n == name).map(|(_, v, _)| *v)
}

// Devices
pub const DSRSENS: usize = 0;
pub const DLRSENS: usize = 1;
pub const DPHASER: usize = 2;
pub const DPHOTON: usize = 3;
pub const DLIFSUP: usize = 4;
pub const DWARPEN: usize = 5;
pub const DIMPULS: usize = 6;
pub const DSHIELD: usize = 7;
pub const DRADIO: usize = 8;
pub const DSHUTTL: usize = 9;
pub const DCOMPTR: usize = 10;
pub const DNAVSYS: usize = 11;
pub const DTRANSP: usize = 12;
pub const DSHCTRL: usize = 13;
pub const DDRAY: usize = 14;
pub const DDSP: usize = 15;
pub const DCLOAK: usize = 16;
pub const NDEVICES: usize = 17;

pub const SKILL_NONE: i64 = 0;
pub const SKILL_NOVICE: i64 = 1;
pub const SKILL_FAIR: i64 = 2;
pub const SKILL_GOOD: i64 = 3;
pub const SKILL_EXPERT: i64 = 4;
pub const SKILL_EMERITUS: i64 = 5;

// Future events
pub const FSPY: usize = 0;
pub const FSNOVA: usize = 1;
pub const FTBEAM: usize = 2;
pub const FSNAP: usize = 3;
pub const FBATTAK: usize = 4;
pub const FCDBAS: usize = 5;
pub const FSCMOVE: usize = 6;
pub const FSCDBAS: usize = 7;
pub const FDSPROB: usize = 8;
pub const FDISTR: usize = 9;
pub const FENSLV: usize = 10;
pub const FREPRO: usize = 11;
pub const NEVENTS: usize = 12;

// Finish codes
pub const FWON: i64 = 0;
pub const FDEPLETE: i64 = 1;
pub const FLIFESUP: i64 = 2;
pub const FNRG: i64 = 3;
pub const FBATTLE: i64 = 4;
pub const FNEG3: i64 = 5;
pub const FNOVA: i64 = 6;
pub const FSNOVAED: i64 = 7;
pub const FABANDN: i64 = 8;
pub const FDILITHIUM: i64 = 9;
pub const FMATERIALIZE: i64 = 10;
pub const FPHASER: i64 = 11;
pub const FLOST: i64 = 12;
pub const FMINING: i64 = 13;
pub const FDPLANET: i64 = 14;
pub const FPNOVA: i64 = 15;
pub const FSSC: i64 = 16;
pub const FSTRACTOR: i64 = 17;
pub const FDRAY: i64 = 18;
pub const FTRIBBLE: i64 = 19;
pub const FHOLE: i64 = 20;
pub const FCREW: i64 = 21;
pub const FCLOAK: i64 = 22;

pub const NSYM: usize = 14;

pub const SYSTNAMES: [&str; 32] = [
    "Andoria (Fesoan)",
    "Tellar Prime (Miracht)",
    "Vulcan (T'Khasi)",
    "Medusa",
    "Argelius II (Nelphia)",
    "Ardana",
    "Catulla (Cendo-Prae)",
    "Gideon",
    "Aldebaran III",
    "Alpha Majoris I",
    "Altair IV",
    "Ariannus",
    "Benecia",
    "Beta Niobe I (Sarpeidon)",
    "Alpha Carinae II",
    "Capella IV (Kohath)",
    "Daran V",
    "Deneb II",
    "Eminiar VII",
    "Gamma Canaris IV",
    "Gamma Tranguli VI (Vaalel)",
    "Ingraham B",
    "Janus IV",
    "Makus III",
    "Marcos XII",
    "Omega IV",
    "Regulus V",
    "Deneva",
    "Rigel II",
    "Beta III",
    "Triacus",
    "Exo III",
];

pub const DEVICE: [&str; NDEVICES] = [
    "S. R. Sensors",
    "L. R. Sensors",
    "Phasers",
    "Photon Tubes",
    "Life Support",
    "Warp Engines",
    "Impulse Engines",
    "Shields",
    "Subspace Radio",
    "Shuttle Craft",
    "Computer",
    "Navigation System",
    "Transporter",
    "Shield Control",
    "Death Ray",
    "D. S. Probe",
    "Cloaking Device",
];

/// The command table, in order; the trailing "" entry is SST2K's.
pub const COMMANDS: &[(&str, i64)] = &[
    ("SRSCAN", OPTION_TTY),
    ("STATUS", OPTION_TTY),
    ("REQUEST", OPTION_TTY),
    ("LRSCAN", OPTION_TTY),
    ("PHASERS", 0),
    ("TORPEDO", 0),
    ("PHOTONS", 0),
    ("MOVE", 0),
    ("SHIELDS", 0),
    ("DOCK", 0),
    ("DAMAGES", 0),
    ("CHART", 0),
    ("IMPULSE", 0),
    ("REST", 0),
    ("WARP", 0),
    ("SENSORS", OPTION_PLANETS),
    ("ORBIT", OPTION_PLANETS),
    ("TRANSPORT", OPTION_PLANETS),
    ("MINE", OPTION_PLANETS),
    ("CRYSTALS", OPTION_PLANETS),
    ("SHUTTLE", OPTION_PLANETS),
    ("PLANETS", OPTION_PLANETS),
    ("REPORT", 0),
    ("COMPUTER", 0),
    ("COMMANDS", 0),
    ("EMEXIT", 0),
    ("PROBE", OPTION_PROBE),
    ("SAVE", 0),
    ("FREEZE", 0),
    ("OPTIONS", 0),
    ("ABANDON", 0),
    ("DESTRUCT", 0),
    ("DEATHRAY", 0),
    ("CAPTURE", OPTION_CAPTURE),
    ("CLOAK", OPTION_CLOAK),
    ("DEBUG", 0),
    ("MAYDAY", 0),
    ("SOS", 0),
    ("CALL", 0),
    ("QUIT", 0),
    ("HELP", 0),
    ("SCORE", 0),
    ("CURSES", 0),
    ("AM", 0),
    ("", 0),
];
