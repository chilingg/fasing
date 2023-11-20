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
import { FORMAT_SYMBOL, CHAR_GROUP_LIST } from "@/lib/construct";

import { invoke } from "@tauri-apps/api/tauri";
import * as dialog from "@tauri-apps/api/dialog";
import { useState, useEffect, useRef } from "react";
import { useImmer } from "use-immer";
import { Item, List } from "@/widgets/List";
import { SimpleCollapsible } from "@/widgets/Collapsible";

import style from "@/styles/CombinationWorkspace.module.css"

const WORK_ID = STORAGE_ID.combWorkspace;

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
    charMembers,
}) {
    function handleCharGroupChange(e, active, value) {
        let list = new Set(charGroup);
        active ? list.add(value) : list.delete(value);
        setCharGroup(list);
    }

    function exportCharList() {
        dialog.save({
            filters: [{
                name: 'svg',
                extensions: ['svg']
            }]
        }).then(path => path && invoke("export_combs", { path, list: charMembers }));
    }

    function exportCharListAll() {
        dialog.open({
            directory: true,
        }).then(path => path && invoke("export_all_combs", { path, size: 1200, strokeWidth: 50, padding: 0, list: charMembers }));
    }

    return (
        <Settings>
            <Vertical>
                <Horizontal>
                    <Input label="过滤" value={filter} setValue={setFilter} />
                    <hr vertical="" />
                    <Selections items={CHAR_GROUP_LIST} currents={charGroup} onChange={handleCharGroupChange} />
                    <Button onClick={() => genCharMembers()}>生成</Button>
                    <Button onClick={() => exportCharList()}>导出</Button>
                    <Button onClick={() => exportCharListAll()}>导出全部</Button>
                </Horizontal>
            </Vertical>
        </Settings>
    )
}

function CombInfos({ info, prefix = "", level = 0 }) {
    if (info.format === "Single") {
        return (
            <div style={{ marginLeft: `${level}em` }}>
                <p>{`${info.name} 长度：${info.trans.h.allocs.length}*${info.trans.v.allocs.length} 等级：${info.trans.h.level}*${info.trans.v.level} ${info.limit ? `限制：${round(info.limit[0])}*${round(info.limit[1])}` : ""}`}</p>
                <table className={style.infoTable}>
                    <tbody>
                        {info.trans.h.allocs.length !== 0 && (<>
                            <tr>
                                <th>横轴</th>
                                {info.trans.h.allocs.map((v, i) => <td key={`${info.name}-allocs-h${i}`}>{v}</td>)}
                            </tr>
                            <tr>
                                <th>&nbsp;</th>
                                {info.trans.h.assign.map((v, i) => <td key={`${info.name}-allocs-h${i}`}>{round(v)}</td>)}
                            </tr>
                        </>)}
                        {info.trans.v.allocs.length !== 0 && (<>
                            <tr>
                                <th>竖轴</th>
                                {info.trans.v.allocs.map((v, i) => <td key={`${info.name}-allocs-v${i}`}>{v}</td>)}
                            </tr>
                            <tr>
                                <th>&nbsp;</th>
                                {info.trans.v.assign.map((v, i) => <td key={`${info.name}-allocs-v${i}`}>{round(v)}</td>)}
                            </tr>
                        </>)}
                    </tbody>
                </table>
            </div>
        )
    } else {
        let name = `${FORMAT_SYMBOL.get(info.format)}${info.comps.map(c => c.name).join("+")}`;
        return (
            <Vertical style={{ marginLeft: `${level}em` }}>
                <p>{`${name} ${info.limit ? `限制：${round(info.limit[0])}*${round(info.limit[1])}` : ""}`}</p>
                <List direction="column">
                    {info.intervals.map((val, i) => <Item key={prefix + name + "interval" + i}>{`${val} ${info.intervals_attr[i]}`}</Item>)}
                </List>
                {info.comps.map((c, i) => <CombInfos key={prefix + c.name + i} info={c} level={level + 1} />)}
            </Vertical>
        )
    }
}

function CharInfo({ char }) {
    const [charInfo, setCharInfo] = useState();

    useEffect(() => {
        invoke("get_char_info", { name: char })
            .then(info => setCharInfo(info))
            .catch(err => console.error(err));
    }, [char]);

    return charInfo
        ? (<div>
            <p>{`h: ${charInfo.white_areas.h[0].toFixed(2)} ${charInfo.white_areas.h[1].toFixed(2)}`}</p>
            <p>{`v: ${charInfo.white_areas.v[0].toFixed(2)} ${charInfo.white_areas.v[1].toFixed(2)}`}</p>
        </div>)
        : <p>{char}</p>
    // charInfo ? <CombInfos info={charInfo} prefix={char} /> : <p>{char}</p>
}

function ConfigSetting({ config, updateConfig }) {
    const CONFIG_ID = WORK_ID.settingPanel.config;

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
                                <th>横轴视级</th>
                                {config.min_values.h.map((v, i) => <td key={`级别${i}`}>{i}</td>)}
                            </tr>
                            <tr>
                                <th>最小值</th>
                                {config.min_values.h.map((v, i) => <td key={`值${i}`}>{v.toFixed(2)}</td>)}
                            </tr>
                            <tr style={TR_STYLE}>
                                <th>竖轴视级</th>
                                {config.min_values.v.map((v, i) => <td key={`级别${i}`}>{i}</td>)}
                            </tr>
                            <tr>
                                <th>最小值</th>
                                {config.min_values.v.map((v, i) => <td key={`值${i}`}>{v.toFixed(2)}</td>)}
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
                <SimpleCollapsible title="视觉重心" storageId={CONFIG_ID.openLimit}>
                    <Vertical>
                        <Horizontal>
                            <Input type="range" label="横轴" value={config.center.h} min={0} max={1} step={0.05} setValue={val => updateConfig(draft => {
                                draft.center.h = Number(val);
                            })}></Input>
                            <p>{config.center.h.toFixed(2)}</p>
                        </Horizontal>
                        <Horizontal>
                            <Input type="range" label="倍率" value={config.center_correction.h} min={-1} max={1} step={0.2} setValue={val => updateConfig(draft => {
                                draft.center_correction.h = Number(val);
                            })}></Input>
                            <p>{config.center_correction.h.toFixed(2)}</p>
                        </Horizontal>
                        <Horizontal>
                            <Input type="range" label="竖轴" value={config.center.v} min={0} max={1} step={0.05} setValue={val => updateConfig(draft => {
                                draft.center.v = Number(val);
                            })}></Input>
                            <p>{config.center.v.toFixed(2)}</p>
                        </Horizontal>
                        <Horizontal>
                            <Input type="range" label="倍率" value={config.center_correction.v} min={-1} max={1} step={0.2} setValue={val => updateConfig(draft => {
                                draft.center_correction.v = Number(val);
                            })}></Input>
                            <p>{config.center_correction.v.toFixed(2)}</p>
                        </Horizontal>
                    </Vertical>
                </SimpleCollapsible>
                <SimpleCollapsible title="中宫" storageId={CONFIG_ID.openLimit}>
                    <Vertical>
                        <Horizontal>
                            <Input type="range" label="横轴" value={config.central_correction.h} min={0} max={10} step={0.1} setValue={val => updateConfig(draft => {
                                draft.central_correction.h = Number(val);
                            })}></Input>
                            <p>{config.central_correction.h.toFixed(2)}</p>
                        </Horizontal>
                        <Horizontal>
                            <Input type="range" label="竖轴" value={config.central_correction.v} min={0} max={2} step={0.1} setValue={val => updateConfig(draft => {
                                draft.central_correction.v = Number(val);
                            })}></Input>
                            <p>{config.central_correction.v.toFixed(2)}</p>
                        </Horizontal>
                    </Vertical>
                </SimpleCollapsible>
                <SimpleCollapsible title="字面" storageId={CONFIG_ID.openLimit}>
                    <Vertical>
                        <Horizontal>
                            <Input type="range" label="横轴" value={config.peripheral_correction.h} min={0} max={10} step={0.1} setValue={val => updateConfig(draft => {
                                draft.peripheral_correction.h = Number(val);
                            })}></Input>
                            <p>{config.peripheral_correction.h.toFixed(2)}</p>
                        </Horizontal>
                        <Horizontal>
                            <Input type="range" label="竖轴" value={config.peripheral_correction.v} min={0} max={2} step={0.1} setValue={val => updateConfig(draft => {
                                draft.peripheral_correction.v = Number(val);
                            })}></Input>
                            <p>{config.peripheral_correction.v.toFixed(2)}</p>
                        </Horizontal>
                    </Vertical>
                </SimpleCollapsible>
            </Vertical>
        )
    } else {
        return <p>无配置</p>
    }
}

function StrokeSetting({ charMembers }) {
    const [strokes, setStrokes] = useState();

    function handelStatistis() {
        invoke("stroke_types", { list: charMembers }).then(response => {
            setStrokes(Object.entries(response));
        })
    }

    return (
        <Vertical>
            <Button onClick={handelStatistis}>统计</Button>
            {strokes?.length && (
                <List direction="column">
                    {strokes.map(stroke => (
                        <Item key={stroke[0]}>
                            <SimpleCollapsible title={`${stroke[0]} - ${stroke[1].length}`} defaultOpen={false}>{stroke[1].join(', ')}</SimpleCollapsible>
                        </Item>
                    ))}
                </List>
            )}
        </Vertical>
    )
}

function WorkspaceSettingPanel({ selects, config, charMembers, updateConfig }) {
    const [openSelect, setOpenSelect] = useState(true);
    const [openConfig, setOpenConfig] = useState(true);
    const [openStroke, setOpenStroke] = useState(true);
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
            setOpen: setOpenConfig,
            component: <ConfigSetting config={config} updateConfig={updateConfig} />,
        },
        {
            id: "stroke",
            title: "笔画",
            open: openStroke,
            setOpen: setOpenStroke,
            component: <StrokeSetting charMembers={charMembers} />
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

    const [config, updateConfigProto] = useImmer();

    const normalOffsetRef = useRef(Context.getItem(WORK_ID.scrollOffset));

    useEffect(() => {
        let group = Context.getItem(WORK_ID.charGroup);
        if (group) {
            setCharGroupProto(group);
            genCharMembersInGroup(group);
        }

        let sele = Context.getItem(WORK_ID.selects);
        sele && setSelectsProto(sele);

        invoke("get_config").then(cfg => updateConfigProto(draft => draft = cfg));
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

    function updateConfig(f) {
        updateConfigProto(draft => {
            f(draft);
            invoke("set_config", { config: draft })
        })
    }

    function genCharMembersInGroup(group = charGroup) {
        let members = [];
        for (const [name, attrs] of constructTab) {
            let tp = attrs.tp;
            if (config && config.correction_table.data.hasOwnProperty(name)) {
                tp = config.correction_table.data[name].tp;
            }
            if (group.has(tp)) {
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

    let charDatas = charMembers;
    if (filter.length != 0) {
        charDatas = filter.split('').filter(c => charMembers.includes(c));
    }
    charDatas = charDatas.map(char => {
        return {
            id: char,
            data: {
                name: char,
                selected: selects.has(char),
                constructTab,
                config,
                setSelected: (sele => {
                    setSelects(sele ? new Set([char]) : new Set());
                })
            }
        }
    });
    // Test
    // let char = "史";
    // charDatas = [{
    //     id: char,
    //     data: {
    //         name: char,
    //         selected: selects.has(char),
    //         constructTab,
    //         config,
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
                    charMembers={charMembers}
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
            <WorkspaceSettingPanel selects={selects} config={config} updateConfig={updateConfig} charMembers={charMembers} />
        </div>
    )
}