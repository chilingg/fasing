import Footer from "./Footer";
import Settings from "./Settings";
import StrucDisplay from "./StrucDisplay";
import SettingPanel from "./SettingPanel";

import { ItemsScrollArea } from "@/widgets/Scroll";
import { ActionBtn, Button, SelectBtn } from "@/widgets/Button";
import { SelectionLabel } from "@/widgets/Selection";
import { Horizontal, Vertical } from "@/widgets/Line";
import { ContentPanel, Tips } from "@/widgets/Menu";
import { HuePicker } from "@/widgets/ColorPicker";
import { List, Item } from "@/widgets/List";
import Input from "@/widgets/Input";

import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";

import { STORAGE_ID, Context } from "@/lib/storageId";
import style from "@/styles/ComponentWorkspace.module.css"
import { useImmer } from "use-immer";
import { SimpleCollapsible } from "@/widgets/Collapsible";

const MARK_OPTIONS = [
    "point",
    "mark",
    "hide",
    "allocate"
];

const FILTER_TYPE_OPTIONS = [
    "empty",
    "valid",
    "single",
    "complex",
]

const WORK_ID = STORAGE_ID.compWorkspace;

function ColorBtn({ color, setColor, switchColor }) {
    const [pos, setPos] = useState(null);

    return (
        <div
            className={style.colorBtn}
            style={{ backgroundColor: color ? `hsl(${color.h}, ${color.s}%, ${color.l}%)` : "transparent" }}
            onClick={(e) => {
                setPos({ left: e.clientX - 160, top: e.clientY })
            }}
        >
            <ContentPanel
                pos={pos}
                setClose={() => setPos(null)}
            >
                <Horizontal style={{ overflow: "visible" }}>
                    <HuePicker hue={color === null ? 0 : color.h} setHue={setColor} disabled={color === null ? "disabled" : null} />
                    <Button onClick={() => switchColor(color === null ? true : false)}>{color === null ? "显示" : "隐藏"}</Button>
                </Horizontal>
            </ContentPanel>

        </div>
    );
}

function AllocateSettings({ allocateTab, setAllocateTab }) {
    return (
        <div>
            <table className={style.allocateTab}>
                <thead>
                    <tr>
                        <th>序号</th>
                        <th><SelectBtn
                            checked={allocateTab.find(rule => rule.disabled) ? false : true}
                            onClick={(e, checked) => {
                                setAllocateTab(allocateTab.map(rule => { return { ...rule, disabled: !checked } }))
                            }}
                        /></th>
                        <th>权重</th>
                        <th>过滤</th>
                        <th>规则</th>
                    </tr>
                </thead>
                <tfoot >
                    <tr>
                        <td className={style.footInfo} colSpan="5">总计：{allocateTab.length}</td>
                    </tr>
                </tfoot>
                <tbody>
                    {allocateTab.map((rule, i) => {
                        return (
                            <tr key={i}>
                                <td>{i}</td>
                                <td>
                                    <SelectBtn
                                        checked={rule.disabled ? false : true}
                                        onClick={(e, checked) => {
                                            let newTab = [...allocateTab];
                                            newTab[i].disabled = !checked;
                                            setAllocateTab(newTab);
                                        }}
                                    />
                                </td>
                                <td>{rule.weight}</td>
                                <td>{rule.filter.length === 0 ? "无" : <Tips tips={rule.filter.join('; ')}>有</Tips>}</td>
                                <td>{rule.regex.source}</td>
                            </tr>
                        )
                    })}
                </tbody>
            </table>
        </div>
    )
}

function AttrMatch({ attr, regex }) {
    const P_STYLE = { display: "inline" };
    const MATCH_STYLE = { textDecoration: "var(--selecte-color) wavy underline 2px" };

    if (regex) {
        let match = attr.match(regex);
        if (match) {
            return (
                <p style={P_STYLE}>
                    {attr.slice(0, match.index)}
                    <span style={MATCH_STYLE}>{match[0]}</span>
                    {attr.slice(match.index + match[0].length)}
                </p>
            )
        } else {
            return <p style={P_STYLE}>{attr}</p>
        }
    } else {
        return <p style={P_STYLE}>{attr}</p>
    }
}

function TestingSettings({ selects, setSelects, setFilterList, testRule, setTestRule }) {
    const [attrsList, updateAttrsList] = useImmer(new Map());
    const [running, setRunning] = useState(false);

    const ITEM_STYLE = { padding: "4px 0 4px 1em", wordBreak: "break-all", lineHeight: 1.6 };

    useEffect(() => {
        updateAttrsList(draft => {
            let deleteList = [...attrsList.keys()].filter(name => !selects.has(name));
            for (let i = 0; i < deleteList.length; ++i) {
                draft.delete(deleteList[i]);
            }
        });

        let names = [...selects].filter(name => !attrsList.has(name));
        if (names.length) {
            invoke("get_struc_attributes", { names })
                .then(list => updateAttrsList(draft => {
                    for (let name in list) {
                        draft.set(name, list[name]);
                    }
                }));
        }
    }, [selects]);

    function handleRunning(active) {
        if (active && testRule.length) {
            try {
                let regex = new RegExp(testRule);
                setTestRule(regex);
                setRunning(true);
            } catch (e) {
                console.error(e)
            }
        } else {
            setRunning(false);
        }
    }

    return (
        <div>
            <Vertical>
                <Horizontal>
                    <ActionBtn active={running}
                        onAction={(e, active) => {
                            handleRunning(active);
                        }}
                    >
                        {running ? "运行" : "测试"}
                    </ActionBtn>
                    <Button
                        disabled={running ? undefined : "disabled"}
                        onClick={() => {
                            if (running) {
                                invoke("fiter_attribute", { regex: testRule.source })
                                    .then(list => setFilterList(list))
                                    .catch(e => console.error(e))
                            }
                        }}
                    >过滤</Button>
                    <Input
                        extension={true}
                        value={testRule?.source || testRule}
                        setValue={str => {
                            if (running) setRunning(false);
                            setTestRule(str);
                        }}
                        onKeyUp={e => {
                            if (e.key === "Enter") {
                                handleRunning(true);
                            }
                        }}
                    />
                </Horizontal>
                <hr />
                <List direction="column">
                    {[...attrsList].map(([name, { h, v }]) => (
                        <Item key={name}>
                            <Horizontal style={{ alignItems: "start" }} spacing={false}>
                                <p style={{ overflow: "visible" }}>{name}</p>
                                <Vertical>
                                    <SimpleCollapsible title={"横轴"} defaultOpem={true}>
                                        <ul style={{ listStyleType: "disc", listStylePosition: "inside" }}>
                                            {h.map((attr, i) => <li key={i} style={ITEM_STYLE}>
                                                <AttrMatch attr={attr} regex={running ? testRule : undefined} />
                                            </li>)}
                                        </ul>
                                    </SimpleCollapsible>
                                    <SimpleCollapsible title={"竖轴"} defaultOpem={true}>
                                        <ul style={{ listStyleType: "disc", listStylePosition: "inside" }}>
                                            {v.map((attr, i) => <li key={i} style={ITEM_STYLE}>
                                                <AttrMatch attr={attr} regex={running ? testRule : undefined} />
                                            </li>)}
                                        </ul>
                                    </SimpleCollapsible>
                                </Vertical>
                            </Horizontal>
                        </Item>
                    ))
                    }
                </List >
            </Vertical>
        </div>
    )
}

function WorkspaceSettingPanel({ allocateTab, setAllocateTab, selects, setSelects, setFilterList, testRule, setTestRule }) {
    const [openAllocate, setOpenAllocate] = useState(true);
    const [openRegexTest, setOpenRegexTest] = useState(true);
    const [width, setWidth] = useState(360);

    useEffect(() => {
        let w = Context.getItem(WORK_ID.settingPanel.width);
        if (w && w > 100) {
            setWidth(w);
        }
    }, []);

    function handleResize(rect) {
        setWidth(rect.width);
        Context.setItem(WORK_ID.settingPanel.width, rect.width);
    }

    let items = [
        {
            id: "allocate",
            title: "空间分配",
            open: openAllocate,
            setOpen: setOpenAllocate,
            component: <AllocateSettings allocateTab={allocateTab} setAllocateTab={setAllocateTab} />
        },
        {
            id: "tests",
            title: "规则测试",
            open: openRegexTest,
            setOpen: setOpenRegexTest,
            component: <TestingSettings selects={selects} setSelects={setSelects} setFilterList={setFilterList} testRule={testRule} setTestRule={setTestRule} />
        },
    ];

    return (
        <SettingPanel
            items={items}
            width={width}
            onResize={handleResize}
        />
    )
}

function WorkspaceSettings({
    filter,
    setFilter,
    markingOption,
    setMarkingOption,
}) {
    let filterOptionList = [
        {
            value: "empty",
            label: "空结构"
        },
        {
            value: "valid",
            label: "非空结构"
        },
        {
            value: "single",
            label: "单字码"
        },
        {
            value: "complex",
            label: "复合码"
        },
    ]

    let markingOptionList = [
        {
            value: "point",
            label: "点注"
        },
        {
            value: "mark",
            label: "标记"
        },
        {
            value: "hide",
            label: "隐线"
        },
        {
            value: "allocate",
            label: "分配"
        }
    ];

    function handleMarkingOptionSet(e, active, value) {
        let newMarkingOp = new Set(markingOption);
        active ? newMarkingOp.add(value) : newMarkingOp.delete(value);
        setMarkingOption(newMarkingOp);
    }

    function handleFilterOptionSet(e, active, value) {
        let newOpiton = new Set(filter.options);
        active ? newOpiton.add(value) : newOpiton.delete(value);
        setFilter({ options: newOpiton, list: filter.list });
    }

    function handleFilterList(str) {
        setFilter({ list: str === '' ? [] : str.split(/ +/), options: filter.options })
    }

    return (
        <Settings>
            <Vertical>
                <Horizontal>
                    <Input label="过滤" value={filter.list.join(' ')} setValue={handleFilterList} />
                    <SelectionLabel items={filterOptionList} currents={filter.options} onChange={handleFilterOptionSet} />
                    <hr vertical="" />
                    <label>显示</label>
                    <SelectionLabel items={markingOptionList} currents={markingOption} onChange={handleMarkingOptionSet} />
                </Horizontal>
            </Vertical>
        </Settings>
    )
}

export default function ComponentsWorkspace({ compList, allocateTab, setAllocateTab }) {
    const [filter, setFilterProto] = useState({
        list: [],
        options: new Set(FILTER_TYPE_OPTIONS),
    });
    const [markingOption, setMarkingOptionProto] = useState(new Set(MARK_OPTIONS));
    const [selects, setSelectsProto] = useState(new Set());
    const [noteRule, setNoteRuleProto] = useState("");

    const normalOffsetRef = useRef(Context.getItem(WORK_ID.scrollOffset));

    useEffect(() => {
        let filter = Context.getItem(WORK_ID.setting.filter);
        filter && setFilterProto(filter);
        let markings = Context.getItem(WORK_ID.setting.markings);
        markings && setMarkingOptionProto(markings);
        let sels = Context.getItem(WORK_ID.selects);
        sels && setSelectsProto(sels);
        let testRule = Context.getItem(WORK_ID.testRule);
        testRule && setNoteRuleProto(testRule);
    }, []);

    function setNoteRule(rule) {
        setNoteRuleProto(rule);
        if (typeof rule === "string") {
            Context.setItem(WORK_ID.testRule, rule);
        } else {
            Context.setItem(WORK_ID.testRule, rule.source);
        }
    }

    function setFilter(filter) {
        setFilterProto(filter);
        Context.setItem(WORK_ID.setting.filter, filter);
    }

    function setMarkingOption(options) {
        setMarkingOptionProto(options);
        Context.setItem(WORK_ID.setting.markings, options);
    }

    function setSelects(sels) {
        setSelectsProto(sels);

        switch (typeof sels) {
            case "object":
                Context.setItem(WORK_ID.selects, sels);
                break;
            case "function":
                Context.setItem(WORK_ID.selects, sels(selects));
        }
    }

    function isFiltering() {
        return filter.list.length || filter.options.size != FILTER_TYPE_OPTIONS.length
    }

    function handleScroll(e) {
        if (!isFiltering()) {
            normalOffsetRef.current = e.target.scrollTop;
            Context.setItem(WORK_ID.scrollOffset, e.target.scrollTop);
        }
    }

    let strucItems = [];
    compList.forEach((struc, name) => {
        if (filter.list.length === 0 || filter.list.indexOf(name) !== -1) {
            if (!filter.options.has("empty") && struc.key_paths.length === 0) {
                return
            }
            if (!filter.options.has("valid") && struc.key_paths.length) {
                return
            }
            if (!filter.options.has("single") && name.length === 1) {
                return
            }
            if (!filter.options.has("complex") && name.length !== 1) {
                return
            }
            strucItems.push({
                id: name,
                data: {
                    name,
                    struc: compList.get(name),
                    markingOption,
                    allocateTab,
                    selected: selects.has(name),
                    noteRule: (typeof noteRule === "object") ? noteRule : undefined,
                    setSelects,
                }
            });
        }
    });

    // Test
    // let name = "㐆";
    // strucItems = [{
    //     id: name,
    //     data: {
    //         name,
    //         struc: compList.get(name),
    //         markingOption,
    //         allocateTab,
    //         selected: selects.has(name),
    //         noteRule: (typeof noteRule === "object") ? noteRule : undefined,
    //         setSelects,
    //     }
    // }];

    return (
        <div style={{ display: "flex", flexDirection: "row", height: "100%" }}>
            <div style={{ display: "flex", flex: "1", flexDirection: "column" }}>
                <WorkspaceSettings
                    filter={filter}
                    setFilter={setFilter}
                    markingOption={markingOption}
                    setMarkingOption={setMarkingOption}
                />
                <div style={{ flex: "1" }}
                    onClick={e => setSelects(new Set())}>
                    <ItemsScrollArea
                        ItemType={StrucDisplay}
                        items={strucItems}
                        onScroll={handleScroll}
                        initOffset={isFiltering() ? 0 : normalOffsetRef.current}
                    />
                </div>
                <Footer>
                    {isFiltering()
                        ? <p>部件<span className="info-text">{` ${strucItems.length}`}</span>{` / ${compList.size}`}</p>
                        : <p>{`部件 ${compList.size}`}</p>
                    }
                </Footer>
            </div>
            <WorkspaceSettingPanel
                allocateTab={allocateTab}
                setAllocateTab={setAllocateTab}
                selects={selects}
                setSelects={setSelects}
                setFilterList={list => setFilter({ options: filter.options, list })}
                testRule={noteRule}
                setTestRule={setNoteRule}
            />
        </div>
    );
}