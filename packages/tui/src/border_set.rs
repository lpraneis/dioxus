pub(crate) struct Set {
    pub horizontal: &'static str,
    pub vertical: &'static str,
    pub bottom_left: &'static str,
    pub top_left: &'static str,
    pub bottom_right: &'static str,
    pub top_right: &'static str,
}

pub(crate) const NORMAL: Set = Set {
    horizontal: "─",
    vertical: "│",
    bottom_left: "└",
    top_left: "┌",
    bottom_right: "┘",
    top_right: "┐",
};

pub(crate) const DOUBLE: Set = Set {
    horizontal: "═",
    vertical: "║",
    bottom_left: "╝",
    top_left: "╗",
    bottom_right: "╚",
    top_right: "╔",
};
