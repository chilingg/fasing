
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
        filter: attrs => attrs.tp.Surround && attrs.tp.Surround.h === "Mind" && attrs.tp.Surround.v === "Start"
    },
    {
        value: "SurroundFromBelow",
        label: "⿶",
        filter: attrs => attrs.tp.Surround && attrs.tp.Surround.h === "Mind" && attrs.tp.Surround.v === "End"
    },
    {
        value: "FullSurround",
        label: "⿴",
        filter: attrs => attrs.tp.Surround && attrs.tp.Surround.h === "Mind" && attrs.tp.Surround.v === "Mind"
    },
    {
        value: "SurroundFromUpperRight",
        label: "⿹",
        filter: attrs => attrs.tp.Surround && attrs.tp.Surround.h === "End" && attrs.tp.Surround.v === "Start"
    },
    {
        value: "SurroundFromLeft",
        label: "⿷",
        filter: attrs => attrs.tp.Surround && attrs.tp.Surround.h === "Start" && attrs.tp.Surround.v === "Mind"
    },
    {
        value: "SurroundFromUpperLeft",
        label: "⿸",
        filter: attrs => attrs.tp.Surround && attrs.tp.Surround.h === "Start" && attrs.tp.Surround.v === "Start"
    },
    {
        value: "SurroundFromLowerLeft",
        label: "⿺",
        filter: attrs => attrs.tp.Surround && attrs.tp.Surround.h === "Start" && attrs.tp.Surround.v === "End"
    },
];

export const FORMAT_SYMBOL = new Map(CHAR_GROUP_LIST.map(({ value, label }) => [value, label]))
