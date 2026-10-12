use crate::core::{ItemDef, NodeDef, ParamDef, ParamKind, settings::{DefaultValue, always_active}};

static P_KB_INFO: ParamDef = ParamDef {
    key: "global.keybind.info",
    label: "Info key",
    description: "Key to open Info sidebar",
    kind: ParamKind::Keybind,
    active: always_active,
    default: DefaultValue::Str("i"),
};

static P_KB_TABS: ParamDef = ParamDef {
    key: "global.keybind.tabs",
    label: "Tabs key",
    description: "Key to open Tabs sidebar",
    kind: ParamKind::Keybind,
    active: always_active,
    default: DefaultValue::Str("t"),
};

static P_KB_BOOKMARKS: ParamDef = ParamDef {
    key: "global.keybind.bookmarks",
    label: "Bookmarks key",
    description: "Key to open Bookmarks sidebar",
    kind: ParamKind::Keybind,
    active: always_active,
    default: DefaultValue::Str("m"),
};

static P_KB_SETTINGS: ParamDef = ParamDef {
    key: "global.keybind.settings",
    label: "Settings key",
    description: "Key to open settings",
    kind: ParamKind::Keybind,
    active: always_active,
    default: DefaultValue::Str("s"),
};

static P_KB_QUIT: ParamDef = ParamDef {
    key: "global.keybind.quit",
    label: "Quit key",
    description: "Key to quit",
    kind: ParamKind::Keybind,
    active: always_active,
    default: DefaultValue::Str("q"),
};

static P_KB_CREATE_TAB: ParamDef = ParamDef {
    key: "global.keybind.create_tab",
    label: "Create tab key",
    description: "Key to create new tab or add bookmark",
    kind: ParamKind::Keybind,
    active: always_active,
    default: DefaultValue::Str("c"),
};

static P_KB_DELETE_TAB: ParamDef = ParamDef {
    key: "global.keybind.delete_tab",
    label: "Delete tab key",
    description: "Key to delete current tab or selected bookmark",
    kind: ParamKind::Keybind,
    active: always_active,
    default: DefaultValue::Str("d"),
};

static P_DELETE_HINT: ParamDef = ParamDef {
    key: "global.keybind.delete_hint",
    label: "Delete confirmation",
    description: "Show a confirmation dialog before deleting a tab.",
    kind: ParamKind::Bool,
    active: always_active,
    default: DefaultValue::Bool(true),
};

static P_KB_NEXT_TAB: ParamDef = ParamDef {
    key: "global.keybind.next_tab",
    label: "Next tab key",
    description: "Key to switch to the next tab (or shift favorite up)",
    kind: ParamKind::Keybind,
    active: always_active,
    default: DefaultValue::Str("n"),
};

static P_KB_PREV_TAB: ParamDef = ParamDef {
    key: "global.keybind.prev_tab",
    label: "Prev tab key",
    description: "Key to switch to the previous tab (or shift favorite down)",
    kind: ParamKind::Keybind,
    active: always_active,
    default: DefaultValue::Str("p"),
};

static P_KB_BACK: ParamDef = ParamDef {
    key: "global.keybind.back",
    label: "Back key",
    description: "Key to navigate back in tab history",
    kind: ParamKind::Keybind,
    active: always_active,
    default: DefaultValue::Str("b"),
};

static P_KB_FORWARD: ParamDef = ParamDef {
    key: "global.keybind.forward",
    label: "Forward key",
    description: "Key to navigate forward in tab history",
    kind: ParamKind::Keybind,
    active: always_active,
    default: DefaultValue::Str("f"),
};

static P_KB_RELOAD: ParamDef = ParamDef {
    key: "global.keybind.reload",
    label: "Reload key",
    description: "Key to reload the current page",
    kind: ParamKind::Keybind,
    active: always_active,
    default: DefaultValue::Str("r"),
};

static P_KB_FAVORITE: ParamDef = ParamDef {
    key: "global.keybind.favorite",
    label: "Favorite key",
    description: "Key to make the current page a favorite (bookmark slot 1-10)",
    kind: ParamKind::Keybind,
    active: always_active,
    default: DefaultValue::Str("v"),
};

macro_rules! fav_param {
    ($name:ident, $key:literal, $label:literal, $default:literal) => {
        static $name: ParamDef = ParamDef {
            key: $key,
            label: $label,
            description: "Key to open the corresponding favorite slot",
            kind: ParamKind::Keybind,
            active: always_active,
            default: DefaultValue::Str($default),
        };
    };
}

fav_param!(P_KB_FAV_1, "global.keybind.favorite_1", "Favorite 1", "1");
fav_param!(P_KB_FAV_2, "global.keybind.favorite_2", "Favorite 2", "2");
fav_param!(P_KB_FAV_3, "global.keybind.favorite_3", "Favorite 3", "3");
fav_param!(P_KB_FAV_4, "global.keybind.favorite_4", "Favorite 4", "4");
fav_param!(P_KB_FAV_5, "global.keybind.favorite_5", "Favorite 5", "5");
fav_param!(P_KB_FAV_6, "global.keybind.favorite_6", "Favorite 6", "6");
fav_param!(P_KB_FAV_7, "global.keybind.favorite_7", "Favorite 7", "7");
fav_param!(P_KB_FAV_8, "global.keybind.favorite_8", "Favorite 8", "8");
fav_param!(P_KB_FAV_9, "global.keybind.favorite_9", "Favorite 9", "9");
fav_param!(P_KB_FAV_10, "global.keybind.favorite_10", "Favorite 10", "0");

const KEYBIND_CHILDREN: &[ItemDef] = &[
    ItemDef::Param(&P_KB_INFO),
    ItemDef::Param(&P_KB_TABS),
    ItemDef::Param(&P_KB_BOOKMARKS),
    ItemDef::Param(&P_KB_CREATE_TAB),
    ItemDef::Param(&P_KB_DELETE_TAB),
    ItemDef::Param(&P_DELETE_HINT),
    ItemDef::Param(&P_KB_NEXT_TAB),
    ItemDef::Param(&P_KB_PREV_TAB),
    ItemDef::Param(&P_KB_BACK),
    ItemDef::Param(&P_KB_FORWARD),
    ItemDef::Param(&P_KB_RELOAD),
    ItemDef::Param(&P_KB_FAVORITE),
    ItemDef::Param(&P_KB_FAV_1),
    ItemDef::Param(&P_KB_FAV_2),
    ItemDef::Param(&P_KB_FAV_3),
    ItemDef::Param(&P_KB_FAV_4),
    ItemDef::Param(&P_KB_FAV_5),
    ItemDef::Param(&P_KB_FAV_6),
    ItemDef::Param(&P_KB_FAV_7),
    ItemDef::Param(&P_KB_FAV_8),
    ItemDef::Param(&P_KB_FAV_9),
    ItemDef::Param(&P_KB_FAV_10),
    ItemDef::Param(&P_KB_SETTINGS),
    ItemDef::Param(&P_KB_QUIT),
];

pub static NODE_KEYBIND: NodeDef = NodeDef {
    key: "global.keybind",
    label: "Keybinds",
    active: always_active,
    children: KEYBIND_CHILDREN,
};