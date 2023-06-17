import Settings from "./Settings";
import Footer from "./Footer";
import SettingPanel from "./SettingPanel";

import { Horizontal, Vertical } from "@/widgets/Line";
import { ItemsScrollArea } from "@/widgets/Scroll";
import CombDisplay from "./CombDisplay";
import Input from "@/widgets/Input";
import { SelectionLabel, Selections } from "@/widgets/Selection";
import { Button } from "@/widgets/Button";

import { Context, STORAGE_ID } from "@/lib/storageId";

import { invoke } from "@tauri-apps/api/tauri";
import { useState, useEffect, useRef } from "react";
import { Item, List } from "@/widgets/List";
import { SimpleCollapsible } from "@/widgets/Collapsible";

import style from "@/styles/CombinationWorkspace.module.css"

const WORK_ID = STORAGE_ID.combWorkspace;

const CHAR_GROUP_LIST = [
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

const FORMAT_SYMBOL = new Map(CHAR_GROUP_LIST.map(({ value, label }) => [value, label]))

function round(num, precision = 2) {
    let mul = Math.pow(10, precision);
    return Math.round(num * mul) / mul;
}

function WorkspaceSettings({
    filter,
    setFilter,
    charGroup,
    setCharGroup,
    genCharMembers,
}) {

    function handleCharGroupChange(e, active, value) {
        let list = new Set(charGroup);
        active ? list.add(value) : list.delete(value);
        setCharGroup(list);
    }

    return (
        <Settings>
            <Vertical>
                <Horizontal>
                    <Input label="过滤" value={filter} setValue={setFilter} />
                    <hr vertical="" />
                    <Selections items={CHAR_GROUP_LIST} currents={charGroup} onChange={handleCharGroupChange} />
                    <Button onClick={() => genCharMembers()}>生成</Button>
                </Horizontal>
            </Vertical>
        </Settings>
    )
}

function CombInfos({ info }) {
    let comp;

    console.log(info)
    console.log(`长度：${info.trans.h.allocs.length}*${info.trans.v.allocs.length} 等级：${info.trans.h.level}*${info.trans.v.level} ${info.limit ? `${info.limit[0]}*${info.limit[0]}` : ""}`)
    if (info.format === "Single") {
        comp = (
            <Vertical spacing={false}>
                <p>{`长度：${info.trans.h.allocs.length}*${info.trans.v.allocs.length} 等级：${info.trans.h.level}*${info.trans.v.level} ${info.limit ? `${info.limit[0]}*${info.limit[0]}` : ""}`}</p>
                <table>
                    <tbody>
                        <tr>
                            <th>横轴</th>
                            {info.trans.h.allocs.map((v, i) => <td key={`${info.name}-allocs-h${i}`}>{v}</td>)}
                        </tr>
                        <tr>
                            <th>&nbsp;</th>
                            {info.trans.h.assign.map((v, i) => <td key={`${info.name}-allocs-h${i}`}>{round(v)}</td>)}
                        </tr>
                        <tr>
                            <th>竖轴</th>
                            {info.trans.v.allocs.map((v, i) => <td key={`${info.name}-allocs-v${i}`}>{v}</td>)}
                        </tr>
                        <tr>
                            <th>&nbsp;</th>
                            {info.trans.v.assign.map((v, i) => <td key={`${info.name}-allocs-v${i}`}>{round(v)}</td>)}
                        </tr>
                    </tbody>
                </table>
            </Vertical>
        )
    }

    return comp;
}

function CharInfo({ char }) {
    const [charInfo, setCharInfo] = useState("");

    useEffect(() => {
        invoke("get_comb_info", { name: char })
            .then(info => setCharInfo(info))
            .catch(err => console.error(err));
    }, [char]);

    return (
        <Horizontal key={char} style={{ alignItems: "start" }}>
            <p>{char}</p>
            {
                typeof charInfo === "object"
                    ? <CombInfos info={charInfo}></CombInfos>
                    : charInfo
            }
        </Horizontal>
    )
}

function ConfigSetting({ }) {
    const CONFIG_ID = WORK_ID.settingPanel.config;

    const [config, setConfig] = useState();
    const [limitChooseFmt, setLimitChooseFmtProto] = useState(Context.getItem(CONFIG_ID.chooseLimitFmt));
    const [replaceChooseFmt, setReplaceChooseFmtProto] = useState(Context.getItem(CONFIG_ID.chooseReplaceFmt));

    function setLimitChooseFmt(fmt) {
        setLimitChooseFmtProto(fmt);
        Context.setItem(CONFIG_ID.chooseLimitFmt, fmt);
    }

    function setReplaceChooseFmt(fmt) {
        setReplaceChooseFmtProto(fmt);
        Context.setItem(CONFIG_ID.chooseReplaceFmt, fmt);
    }

    useState(() => {
        invoke("get_config").then(cfg => setConfig(cfg));
    }, []);

    const TR_STYLE = { borderBottom: "1px solid var(--inaction-bg-color)" };

    let limitSelectItems = [];
    if (config) {
        for (let fmt in config.format_limit) {
            limitSelectItems.push({
                label: FORMAT_SYMBOL.get(fmt),
                value: fmt
            });
        }
    }
    let replaceSelectItems = [];
    if (config) {
        for (let fmt in config.replace_list) {
            replaceSelectItems.push({
                label: FORMAT_SYMBOL.get(fmt),
                value: fmt
            });
        }
    }

    if (config) {
        return (
            <Vertical>
                <div>
                    <table>
                        <tbody>
                            <tr style={TR_STYLE}>
                                <th>视级</th>
                                {config.min_values.map((v, i) => <td key={`级别${i}`}>{i}</td>)}
                            </tr>
                            <tr>
                                <th>最小值</th>
                                {config.min_values.map((v, i) => <td key={`值${i}`}>{v.toFixed(2)}</td>)}
                            </tr>
                        </tbody>
                    </table>
                    <hr />
                    <table>
                        <tbody>
                            <tr style={TR_STYLE}>
                                <th>等级</th>
                                {config.assign_values.map((v, i) => <td key={`级别${i}`}>{i}</td>)}
                            </tr>
                            <tr>
                                <th>分配值</th>
                                {config.assign_values.map((v, i) => <td key={`值${i}`}>{v.toFixed(2)}</td>)}
                            </tr>
                        </tbody>
                    </table>
                    <hr />
                </div>
                <SimpleCollapsible title="间隔" storageId={CONFIG_ID.openInterval}>
                    <table>
                        <thead>
                            <tr>
                                <th style={{ width: 24 }}>值</th>
                                <th>规则</th>
                            </tr>
                        </thead>
                        <tbody className={style.table}>
                            {config.interval_rule.map((rule, i) => (
                                <tr key={i}>
                                    <td>{Math.round(rule.weight * 100) / 100}</td>
                                    <td style={{ textAlign: "left", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{rule.regex}</td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </SimpleCollapsible>
                <SimpleCollapsible title="格式限制" storageId={CONFIG_ID.openLimit}>
                    <Vertical>
                        <Horizontal style={{ paddingTop: 8 }}>
                            <SelectionLabel
                                items={limitSelectItems}
                                currents={new Set([limitChooseFmt])}
                                onChange={(e, active, val) => active && setLimitChooseFmt(val)}
                            />
                        </Horizontal>
                        {
                            limitChooseFmt && [...Object.entries(config.format_limit[limitChooseFmt])].map(([inFmt, groups]) => {
                                let id = `limit-${FORMAT_SYMBOL.get(limitChooseFmt)}-${inFmt}-open`;
                                return (
                                    <SimpleCollapsible
                                        key={id}
                                        title={`${FORMAT_SYMBOL.get(limitChooseFmt)} ${inFmt}`}
                                        storageId={id}
                                    >
                                        {groups.map(([group, size], i) => {
                                            let groupId = `${id}-group${i}`;
                                            return (
                                                <SimpleCollapsible key={groupId} title={`组${i}: 宽 ${round(size[0])} 高 ${round(size[1])}`} storageId={groupId}>
                                                    <p>{group.join(", ")}</p>
                                                </SimpleCollapsible>
                                            )
                                        })}
                                    </SimpleCollapsible>
                                )
                            })
                        }
                    </Vertical>
                </SimpleCollapsible>
                <SimpleCollapsible title="部件映射" storageId={CONFIG_ID.openReplace}>
                    <Vertical>
                        <Horizontal style={{ paddingTop: 8 }}>
                            <SelectionLabel
                                items={replaceSelectItems}
                                currents={new Set([replaceChooseFmt])}
                                onChange={(e, active, val) => active && setReplaceChooseFmt(val)}
                            />
                        </Horizontal>
                        {
                            replaceChooseFmt && Object.entries(config.replace_list[replaceChooseFmt]).map(([inFmt, maps]) => {
                                let id = `replace-${FORMAT_SYMBOL.get(replaceChooseFmt)}-${inFmt}-open`;
                                return (
                                    <SimpleCollapsible
                                        key={id}
                                        title={`${FORMAT_SYMBOL.get(replaceChooseFmt)} ${inFmt}`}
                                        storageId={id}
                                    >
                                        <List direction="column">
                                            {Object.entries(maps).map(([from, to], i) => (
                                                <Item key={i} style={{ margin: "4px 0" }}>{`${from} -> ${to}`}</Item>
                                            ))}
                                        </List>
                                    </SimpleCollapsible>
                                )
                            })
                        }
                    </Vertical>
                </SimpleCollapsible>
            </Vertical>
        )
    } else {
        return <p>无配置</p>
    }
}

function WorkspaceSettingPanel({ selects }) {
    const [openSelect, setOpenSelect] = useState(true);
    const [openConfig, setConfig] = useState(true);
    const [width, setWidth] = useState(360);

    useEffect(() => {
        let w = Context.getItem(WORK_ID.settingPanel.width);
        if (w && w > 100) {
            setWidth(w);
        }
    }, []);

    let items = [
        {
            id: "select",
            title: "选中",
            open: openSelect,
            setOpen: setOpenSelect,
            component: (
                selects.size ? <CharInfo char={selects.values().next().value} /> : <p>未选中</p>
            )
        },
        {
            id: "config",
            title: "配置",
            open: openConfig,
            setOpen: setConfig,
            component: <ConfigSetting />
        },
    ];

    function handleResize(rect) {
        setWidth(rect.width);
        Context.setItem(WORK_ID.settingPanel.width, rect.width);
    }

    return (
        <SettingPanel
            items={items}
            width={width}
            onResize={handleResize}
        />
    )
}

export default function CombinationWorkspace({ constructTab }) {
    const [charGroup, setCharGroupProto] = useState(new Set(["Single"]));
    const [filter, setFilter] = useState("");
    const [charMembers, setCharMembers] = useState([]);
    const [selects, setSelectsProto] = useState(new Set());

    const normalOffsetRef = useRef(Context.getItem(WORK_ID.scrollOffset));

    useEffect(() => {
        let group = Context.getItem(WORK_ID.charGroup);
        if (group) {
            setCharGroupProto(group);
            genCharMembersInGroup(group);
        }

        let sele = Context.getItem(WORK_ID.selects);
        sele && setSelectsProto(sele);
    }, []);

    useEffect(() => {
        genCharMembersInGroup();
    }, [constructTab])

    function setCharGroup(group) {
        setCharGroupProto(group);
        Context.setItem(WORK_ID.charGroup, group);
    }

    function setSelects(targets) {
        setSelectsProto(targets);
        Context.setItem(WORK_ID.selects, targets);
    }

    function genCharMembersInGroup(group = charGroup) {
        let members = [];
        for (const [name, attrs] of constructTab) {
            if (group.has(attrs.format)) {
                members.push(name)
            }
        }
        setCharMembers(members);
    }

    function handleScroll(e) {
        if (filter.length === 0) {
            normalOffsetRef.current = e.target.scrollTop;
            Context.setItem(WORK_ID.scrollOffset, e.target.scrollTop);
        }
    }

    let charDatas = charMembers.map(char => {
        return {
            id: char,
            data: {
                name: char,
                selected: selects.has(char),
                constructTab,
                setSelected: (sele => {
                    setSelects(sele ? new Set([char]) : new Set());
                })
            }
        }
    });
    // Test
    // let char = "一";
    // charDatas = [{
    //     id: char,
    //     data: {
    //         name: char,
    //         selected: selects.has(char),
    //         constructTab,
    //         setSelected: (sele => {
    //             setSelects(sele ? new Set([char]) : new Set());
    //         })
    //     }
    // }];

    return (
        <div style={{ display: "flex", flexDirection: "row", height: "100%" }}>
            <div style={{ display: "flex", flex: "1", flexDirection: "column" }}>
                <WorkspaceSettings
                    filter={filter}
                    setFilter={setFilter}
                    charGroup={charGroup}
                    setCharGroup={setCharGroup}
                    genCharMembers={genCharMembersInGroup}
                />
                <div style={{ flex: "1" }}>
                    <ItemsScrollArea
                        ItemType={CombDisplay}
                        items={charDatas}
                        onScroll={handleScroll}
                        initOffset={filter.length === 0 ? normalOffsetRef.current : 0}
                    />
                </div>
                <Footer>
                    <p>{`${charMembers.length} 字符`}</p>
                </Footer>
            </div>
            <WorkspaceSettingPanel selects={selects} />
        </div>
    )
}