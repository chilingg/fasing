const MODF_CTRL = 1;
const MODF_SHIFT = 2;
const MODF_ALT = 4;

export const SHORTCUT = {
    save: {
        code: "KeyS",
        modf: MODF_CTRL
    },
    save_as: {
        code: "KeyS",
        modf: MODF_CTRL | MODF_SHIFT
    },
    open: {
        code: "KeyO",
        modf: MODF_CTRL
    },
    reload: {
        code: "KeyR",
        modf: MODF_CTRL
    }
}

export function isKeydown(e, shortcut) {
    return e.code == shortcut.code
        && e.altKey == (shortcut.modf & MODF_ALT)
        && e.shiftKey == (shortcut.modf & MODF_SHIFT)
        && e.ctrlKey == (shortcut.modf & MODF_CTRL)
}

function simpleCode(code) {
    let match = code.match(/Key(.*)/);
    return match[1] || code;
}

export function shortcutText(shortcut) {
    return ((shortcut.modf & MODF_CTRL) ? "Ctrl+" : "")
        + ((shortcut.modf & MODF_SHIFT) ? "Shift+" : "")
        + ((shortcut.modf & MODF_ALT) ? "Alt+" : "")
        + simpleCode(shortcut.code);
}
