
export const CHAR_GROUP_LIST = [
    {
        value: "Single",
        label: "单体",
        filter: attrs => attrs.tp === "Single"
    },
    {
        value: "LeftToRight",
        label: "⿰",
        filter: attrs => attrs.tp.Scale == "Horizontal" && attrs.components.length == 2
    },
    {
        value: "LeftToMiddleAndRight",
        label: "⿲",
        filter: attrs => attrs.tp.Scale == "Horizontal" && attrs.components.length == 3
    },
    {
        value: "AboveToBelow",
        label: "⿱",
        filter: attrs => attrs.tp.Scale == "Vertical" && attrs.components.length == 2
    },
    {
        value: "AboveToMiddleAndBelow",
        label: "⿳",
        filter: attrs => attrs.tp.Scale == "Vertical" && attrs.components.length == 3
    },
    {
        value: "SurroundFromAbove",
        label: "⿵",
        filter: attrs => attrs.tp == { Surround: { h: "Mind", v: "Start" } } && attrs.components.length == 3
    },
    {
        value: "SurroundFromBelow",
        label: "⿶",
        filter: attrs => attrs.tp == { Surround: { h: "Mind", v: "End" } } && attrs.components.length == 3
    },
    {
        value: "FullSurround",
        label: "⿴",
        filter: attrs => attrs.tp == { Surround: { h: "Mind", v: "Mind" } } && attrs.components.length == 3
    },
    {
        value: "SurroundFromUpperRight",
        label: "⿹",
        filter: attrs => attrs.tp == { Surround: { h: "End", v: "Start" } } && attrs.components.length == 3
    },
    {
        value: "SurroundFromLeft",
        label: "⿷",
        filter: attrs => attrs.tp == { Surround: { h: "Start", v: "Mind" } } && attrs.components.length == 3
    },
    {
        value: "SurroundFromUpperLeft",
        label: "⿸",
        filter: attrs => attrs.tp == { Surround: { h: "Start", v: "Start" } } && attrs.components.length == 3
    },
    {
        value: "SurroundFromLowerLeft",
        label: "⿺",
        filter: attrs => attrs.tp == { Surround: { h: "Start", v: "End" } } && attrs.components.length == 3
    },
];

export const FORMAT_SYMBOL = new Map(CHAR_GROUP_LIST.map(({ value, label }) => [value, label]))
