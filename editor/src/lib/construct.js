export const CHAR_GROUP_LIST = [
    {
        value: "Single",
        label: "单",
        filter: attrs => attrs === ""
    },
    {
        value: "LeftToRight",
        label: "⿰",
        filter: attrs => attrs.tp === "⿰" && attrs.components.length == 2
    },
    {
        value: "LeftToMiddleAndRight",
        label: "⿲",
        filter: attrs => attrs.tp === "⿰" && attrs.components.length == 3
    },
    {
        value: "AboveToBelow",
        label: "⿱",
        filter: attrs => attrs.tp === "⿱" && attrs.components.length == 2
    },
    {
        value: "AboveToMiddleAndBelow",
        label: "⿳",
        filter: attrs => attrs.tp === "⿱" && attrs.components.length == 3
    },
    {
        value: "SurroundFromAbove",
        label: "⿵",
        filter: attrs => attrs.tp === "⿵"
    },
    {
        value: "SurroundFromBelow",
        label: "⿶",
        filter: attrs => attrs.tp === "⿶"
    },
    {
        value: "FullSurround",
        label: "⿴",
        filter: attrs => attrs.tp === "⿴"
    },
    {
        value: "SurroundFromUpperRight",
        label: "⿹",
        filter: attrs => attrs.tp === "⿹"
    },
    {
        value: "SurroundFromLeft",
        label: "⿷",
        filter: attrs => attrs.tp === "⿷"
    },
    {
        value: "SurroundFromUpperLeft",
        label: "⿸",
        filter: attrs => attrs.tp === "⿸"
    },
    {
        value: "SurroundFromLowerLeft",
        label: "⿺",
        filter: attrs => attrs.tp === "⿺"
    },
    // {
    //     value: "Letter",
    //     label: "A",
    //     filter: attrs => false
    // },
    // {
    //     value: "Number",
    //     label: "0",
    //     filter: attrs => false
    // },
];

export const TYPE_FILTERS = new Map(CHAR_GROUP_LIST.map(({ value, filter }) => [value, filter]))
