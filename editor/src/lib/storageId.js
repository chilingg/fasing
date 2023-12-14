const { stringify, parse } = JSON
JSON.stringify = function (value, replacer, space) {
    const _replacer =
        typeof replacer === 'function'
            ? replacer
            : function (_, value) {
                return value
            }
    replacer = function (key, value) {
        value = _replacer(key, value)
        if (value instanceof Set) value = `Set{${stringify([...value])}}`
        else if (value instanceof Map) value = `Map{${stringify([...value])}}`
        return value
    }
    return stringify(value, replacer, space)
}
JSON.parse = function (value, reviver) {
    if (!reviver)
        reviver = function (key, value) {
            if (/Set\{\[.*\]\}/.test(value))
                value = new Set(parse(value.replace(/Set\{\[(.*)\]\}/, '[$1]')))
            else if (/Map\{\[.*\]\}/.test(value))
                value = new Map(parse(value.replace(/Map\{\[(.*)\]\}/, '[$1]')))
            return value
        }
    return parse(value, reviver)
} // 作者：死皮赖脸的喵子 https://www.bilibili.com/read/cv20325492 出处：bilibili

export const STORAGE_ID = {
    workStage: "workStage",
    compWorkspace: {
        setting: {
            filter: "cpwk-filter",
            markings: "cpwk-markings",
            allocation: "cpwk-allocation",
        },
        settingPanel: {
            width: "cpwk-panel-width",
        },
        scrollOffset: "cpwk-offset",
        selects: "cpwk-selects",
        testRule: "cpwk-test-rule"
    },
    combWorkspace: {
        settingPanel: {
            width: "cbwk-panel-width",
            config: {
                openInterval: "cbwk-panel-open-interval",
                openLimit: "cbwk-panel-open-limit",
                openReplace: "cbwk-panel-open-replace",

                chooseLimitFmt: "cbwk-panel-limit-fmt",
                chooseReplaceFmt: "cbwk-panel-replace-fmt",
            }
        },
        filter: "cbwk-filter",
        charGroup: "cbwk-group",
        selects: "cbwl_selects",
        scrollOffset: "cbwk-offset",
    }
};

function getContextItem(id) {
    try {
        return JSON.parse(localStorage.getItem(id));
    } catch (e) {
        console.error(e)
        return null
    }
}

function setContextItem(id, value) {
    return localStorage.setItem(id, JSON.stringify(value));
}

export const Context = {
    getItem: getContextItem,
    setItem: setContextItem,
}