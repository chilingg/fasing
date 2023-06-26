
export const CHAR_GROUP_LIST = [
    {
        value: "Single",
        label: "单体"
    },
    {
        value: "LeftToRight",
        label: "⿰"
    },
    {
        value: "LeftToMiddleAndRight",
        label: "⿲"
    },
    {
        value: "AboveToBelow",
        label: "⿱"
    },
    {
        value: "AboveToMiddleAndBelow",
        label: "⿳"
    },
    {
        value: "SurroundFromAbove",
        label: "⿵"
    },
    {
        value: "SurroundFromBelow",
        label: "⿶"
    },
    {
        value: "FullSurround",
        label: "⿴"
    },
    {
        value: "SurroundFromUpperRight",
        label: "⿹"
    },
    {
        value: "SurroundFromLeft",
        label: "⿷"
    },
    {
        value: "SurroundFromUpperLeft",
        label: "⿸"
    },
    {
        value: "SurroundFromLowerLeft",
        label: "⿺"
    },
];

export const FORMAT_SYMBOL = new Map(CHAR_GROUP_LIST.map(({ value, label }) => [value, label]))
