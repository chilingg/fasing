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

    function randomFilter() {
        const CHAR_NUM = 57;
        const MAX_GEN = 2 * CHAR_NUM;

        let rn_list = new Set();
        for (let i = 0; i < MAX_GEN; ++i) {
            rn_list.add(Math.floor(Math.random() * charMembers.length))
            if (rn_list.size === CHAR_NUM) { break }
        }
        let rn_chars = [];
        rn_list.forEach(i => rn_chars.push(charMembers[i]));
        setFilter(rn_chars);
    }

    function exportCharList() {
        dialog.save({
            filters: [{
                name: 'svg',
                extensions: ['svg']
            }]
        }).then(path => path && invoke("export_combs", { path, list: filter.length == 0 ? charMembers : filter }));
    }

    function exportCharDatas() {
        dialog.save({
            filters: [{
                name: 'json',
                extensions: ['json']
            }]
        }).then(path => path && invoke("export_comb_datas", { path, list: filter.length == 0 ? charMembers : filter }));
    }

    function exportCharListAll() {
        dialog.open({
            directory: true,
        }).then(path => path && invoke("export_all_combs", { path, size: 1024, strokeWidth: 64, padding: 0, list: filter.length == 0 ? charMembers : filter }));
    }

    return (
        <Settings>
            <Vertical>
                <Horizontal>
                    <Input label="过滤" value={filter.join('')} setValue={setFilter} />
                    <Button onClick={() => randomFilter()}>随机</Button>
                    <hr vertical="" />
                    <Selections items={CHAR_GROUP_LIST} currents={charGroup} onChange={handleCharGroupChange} />
                    <Button onClick={() => genCharMembers()}>生成</Button>
                    <Button onClick={() => exportCharList()}>导出</Button>
                    <Button onClick={() => exportCharDatas()}>导出数据</Button>
                    <Button onClick={() => exportCharListAll()}>导出全部</Button>
                </Horizontal>
            </Vertical>
        </Settings>
    )
}

function CombInfos({ info, prefix = "", level = 0 }) {
    function sum(a, b) {
        return a + b
    }

    if (info.tp === "Single") {
        return (<table className={style.infoTable}>
            <caption>{`${info.name} 长度：${info.bases.h.reduce(sum, 0)}*${info.bases.v.reduce(sum, 0)}`}</caption>
            <tbody>
                {info.bases.h.length !== 0 && (<>
                    <tr>
                        <th>横轴</th>
                        {info.bases.h.map((v, i) => <td key={`${info.name}-allocs-h${i}`}>{v}</td>)}
                    </tr>
                    <tr>
                        <th>&nbsp;</th>
                        {info.assign.h.map((v, i) => <td key={`${info.name}-allocs-h${i}`}>{round(v)}</td>)}
                    </tr>
                </>)}
                {info.bases.vlength !== 0 && (<>
                    <tr>
                        <th>竖轴</th>
                        {info.bases.v.map((v, i) => <td key={`${info.name}-allocs-v${i}`}>{v}</td>)}
                    </tr>
                    <tr>
                        <th>&nbsp;</th>
                        {info.assign.v.map((v, i) => <td key={`${info.name}-allocs-v${i}`}>{round(v)}</td>)}
                    </tr>
                </>)}
            </tbody>
        </table>)
    } else {
        function list(info) {
            if ("Scale" in info.tp) {
                if (info.tp.Scale === "Horizontal") {
                    return (<List direction="column">
                        {info.bases.h.map((val, i) => <Item key={prefix + info.name + "interval" + i}>
                            <p>{info.i_attr.h[i]}</p>
                            <p>{`${val} ${info.i_notes.h[i]}`}</p>
                        </Item>)}
                    </List>);
                } else {
                    return (<List direction="column">
                        {info.bases.v.map((val, i) => <Item key={prefix + info.name + "interval" + i}>
                            <p>{info.i_attr.v[i]}</p>
                            <p>{`${val} ${info.i_notes.v[i]}`}</p>
                        </Item>)}
                    </List>);
                }
            } else {
                return (<List direction="column">
                    {info.bases.h.map((val, i) => <Item key={prefix + info.name + "hinterval" + i}>
                        <p>{info.i_attr.h[i]}</p>
                        <p>{`${val} ${info.i_notes.h[i]}`}</p>
                    </Item>)}
                    {info.bases.v.map((val, i) => <Item key={prefix + info.name + "vinterval" + i}>
                        <p>{info.i_attr.v[i]}</p>
                        <p>{`${val} ${info.i_notes.v[i]}`}</p>
                    </Item>)}
                </List>)
            }
        }

        let info_list = list(info)
        return (
            <Vertical style={{ marginLeft: `${level}em` }}>
                <p>{info.name}</p>
                {info_list}
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
        ? (<Vertical>
            <p>{charInfo.comb_info}</p>
            <p>{`等级: ${charInfo.level.h} - ${charInfo.level.v}`}</p>
            <p>{`余量比: ${charInfo.scale.h.toFixed(3)} - ${charInfo.scale.v.toFixed(3)}`}</p>
            <p>{`白边: h ${charInfo.white_areas.h[0].toFixed(2)} ${charInfo.white_areas.h[1].toFixed(2)}`}</p>
            <p>{`白边: v ${charInfo.white_areas.v[0].toFixed(2)} ${charInfo.white_areas.v[1].toFixed(2)}`}</p>
            <p>{`视觉重心: (${charInfo.center[0].h?.toFixed(2)} ${charInfo.center[0].v?.toFixed(2)}) -> (${charInfo.center[1].h.toFixed(2)} ${charInfo.center[1].v.toFixed(2)})`}</p>
            <List direction="column">
                {
                    charInfo.comp_infos.map((ci, i) => <Item key={ci.name + i}>
                        <hr />
                        <CombInfos info={ci} prefix={ci.name} />
                    </Item>)
                }
            </List>
        </Vertical >)
        : <p>{char}</p>
}

function ConfigSetting({ config, updateConfig }) {
    const CONFIG_ID = WORK_ID.settingPanel.config;

    const [limitChooseFmt, setLimitChooseFmtProto] = useState(Context.getItem(CONFIG_ID.chooseLimitFmt));
    const [replaceChooseFmt, setReplaceChooseFmtProto] = useState(Context.getItem(CONFIG_ID.chooseReplaceFmt));

    const [hcenter, setHCenter] = useState();
    const [vcenter, setVCenter] = useState();

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
                                    <td>{Math.round(rule.val * 100) / 100}</td>
                                    <td style={{ textAlign: "left", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{rule.regex}</td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </SimpleCollapsible>
                <SimpleCollapsible title="视觉重心" storageId={CONFIG_ID.openLimit}>
                    <Vertical>
                        <Horizontal>
                            <Input disabled={!config.center.h} type="range" label="横轴" value={config.center.h} min={0} max={1} step={0.05} setValue={val => updateConfig(draft => {
                                draft.center.h = Number(val);
                            })}></Input>
                            <p>{config.center.h ? config.center.h.toFixed(2) : "0.00"}</p>
                            <Button onClick={() => {
                                if (config?.center?.h) {
                                    setHCenter(config.center.h);
                                    updateConfig(draft => draft.center.h = null);
                                } else {
                                    updateConfig(draft => draft.center.h = hcenter ? hcenter : 0.5);
                                }
                            }}> {config?.center?.h ? "禁用" : "启用"}</Button>
                        </Horizontal>
                        <Horizontal>
                            <Input type="range" label="倍率" value={config.center_correction.h} min={-1} max={1} step={0.1} setValue={val => updateConfig(draft => {
                                draft.center_correction.h = Number(val);
                            })}></Input>
                            <p>{config.center_correction.h.toFixed(2)}</p>
                        </Horizontal>
                        <Horizontal>
                            <Input disabled={!config.center.v} type="range" label="竖轴" value={config.center.v} min={0} max={1} step={0.05} setValue={val => updateConfig(draft => {
                                draft.center.v = Number(val);
                            })}></Input>
                            <p>{config.center.v ? config.center.v.toFixed(2) : "0.00"}</p>
                            <Button onClick={() => {
                                if (config?.center?.v) {
                                    setVCenter(config.center.v);
                                    updateConfig(draft => draft.center.v = null);
                                } else {
                                    updateConfig(draft => draft.center.v = vcenter ? vcenter : 0.40);
                                }
                            }}> {config?.center?.v ? "禁用" : "启用"}</Button>
                        </Horizontal>
                        <Horizontal>
                            <Input type="range" label="倍率" value={config.center_correction.v} min={-1} max={1} step={0.1} setValue={val => updateConfig(draft => {
                                draft.center_correction.v = Number(val);
                            })}></Input>
                            <p>{config.center_correction.v.toFixed(2)}</p>
                        </Horizontal>
                    </Vertical>
                </SimpleCollapsible>
                <SimpleCollapsible title="部件重心" storageId={CONFIG_ID.openLimit}>
                    <Vertical>
                        <Horizontal>
                            <Input disabled={!config.comp_center.h} type="range" label="横轴" value={config.comp_center.h} min={0} max={1} step={0.05} setValue={val => updateConfig(draft => {
                                draft.comp_center.h = Number(val);
                            })}></Input>
                            <p>{config.comp_center.h ? config.comp_center.h.toFixed(2) : "0.00"}</p>
                            <Button onClick={() => {
                                if (config?.comp_center?.h) {
                                    setHCenter(config.comp_center.h);
                                    updateConfig(draft => draft.comp_center.h = null);
                                } else {
                                    updateConfig(draft => draft.comp_center.h = hcenter ? hcenter : 0.5);
                                }
                            }}> {config?.comp_center?.h ? "禁用" : "启用"}</Button>
                        </Horizontal>
                        <Horizontal>
                            <Input type="range" label="倍率" value={config.comp_center_correction.h} min={-1} max={1} step={0.1} setValue={val => updateConfig(draft => {
                                draft.comp_center_correction.h = Number(val);
                            })}></Input>
                            <p>{config.comp_center_correction.h.toFixed(2)}</p>
                        </Horizontal>
                        <Horizontal>
                            <Input disabled={!config.comp_center.v} type="range" label="竖轴" value={config.comp_center.v} min={0} max={1} step={0.05} setValue={val => updateConfig(draft => {
                                draft.comp_center.v = Number(val);
                            })}></Input>
                            <p>{config.comp_center.v ? config.comp_center.v.toFixed(2) : "0.00"}</p>
                            <Button onClick={() => {
                                if (config?.comp_center?.v) {
                                    setVCenter(config.comp_center.v);
                                    updateConfig(draft => draft.comp_center.v = null);
                                } else {
                                    updateConfig(draft => draft.comp_center.v = vcenter ? vcenter : 0.40);
                                }
                            }}> {config?.comp_center?.v ? "禁用" : "启用"}</Button>
                        </Horizontal>
                        <Horizontal>
                            <Input type="range" label="倍率" value={config.comp_center_correction.v} min={-1} max={1} step={0.1} setValue={val => updateConfig(draft => {
                                draft.comp_center_correction.v = Number(val);
                            })}></Input>
                            <p>{config.comp_center_correction.v.toFixed(2)}</p>
                        </Horizontal>
                    </Vertical>
                </SimpleCollapsible>
                <SimpleCollapsible title="中宫" storageId={CONFIG_ID.openLimit}>
                    <Vertical>
                        <Horizontal>
                            <Input type="range" label="横轴" value={config.central_correction.h} min={0} max={2} step={0.1} setValue={val => updateConfig(draft => {
                                draft.central_correction.h = Number(val);
                            })}></Input>
                            <p>{config.central_correction.h.toFixed(2)}</p>
                            <Button onClick={() => {
                                updateConfig(draft => draft.cp_trigger = !draft.cp_trigger);
                            }}> {config?.cp_trigger ? "禁用" : "启用"}</Button>
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
                            <Input type="range" label="横轴" value={config.peripheral_correction.h} min={0} max={2} step={0.1} setValue={val => updateConfig(draft => {
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
                <SimpleCollapsible title="边缘对齐" storageId={CONFIG_ID.openLimit}>
                    <Vertical>
                        <Horizontal>
                            <Input type="range" label="横轴" value={config.align_edge.h} min={-1} max={1} step={0.1} setValue={val => updateConfig(draft => {
                                draft.align_edge.h = Number(val);
                            })}></Input>
                            <p>{config.align_edge.h.toFixed(2)}</p>
                        </Horizontal>
                        <Horizontal>
                            <Input type="range" label="竖轴" value={config.align_edge.v} min={-1} max={1} step={0.1} setValue={val => updateConfig(draft => {
                                draft.align_edge.v = Number(val);
                            })}></Input>
                            <p>{config.align_edge.v.toFixed(2)}</p>
                        </Horizontal>
                    </Vertical>
                </SimpleCollapsible>
                <SimpleCollapsible title="包围缩放" storageId={CONFIG_ID.openLimit}>
                    <Vertical>
                        <Horizontal>
                            <Input type="range" label="横轴-前" value={config.surround_align.h['Start']} min={-1} max={1} step={0.1} setValue={val => updateConfig(draft => {
                                draft.surround_align.h['Start'] = Number(val);
                            })}></Input>
                            <p>{config.surround_align.h['Start'].toFixed(2)}</p>
                        </Horizontal>
                        <Horizontal>
                            <Input type="range" label="横轴-后" value={config.surround_align.h['End']} min={-1} max={1} step={0.1} setValue={val => updateConfig(draft => {
                                draft.surround_align.h['End'] = Number(val);
                            })}></Input>
                            <p>{config.surround_align.h['End'].toFixed(2)}</p>
                        </Horizontal>
                        <Horizontal>
                            <Input type="range" label="竖轴-前" value={config.surround_align.v['Start']} min={-1} max={1} step={0.1} setValue={val => updateConfig(draft => {
                                draft.surround_align.v['Start'] = Number(val);
                            })}></Input>
                            <p>{config.surround_align.v['Start'].toFixed(2)}</p>
                        </Horizontal>
                        <Horizontal>
                            <Input type="range" label="竖轴-后" value={config.surround_align.v['End']} min={-1} max={1} step={0.1} setValue={val => updateConfig(draft => {
                                draft.surround_align.v['End'] = Number(val);
                            })}></Input>
                            <p>{config.surround_align.v['End'].toFixed(2)}</p>
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
            id: "config",
            title: "配置",
            open: openConfig,
            setOpen: setOpenConfig,
            component: <ConfigSetting config={config} updateConfig={updateConfig} />,
        },
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
    const [filter, setFilterProto] = useState([]);
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

        let filter = Context.getItem(WORK_ID.filter);
        filter && setFilterProto(filter);

        let sele = Context.getItem(WORK_ID.selects);
        sele && setSelectsProto(sele);

        invoke("get_config").then(cfg => updateConfigProto(draft => draft = cfg));
    }, []);

    useEffect(() => {
        genCharMembersInGroup();
    }, [constructTab])

    function setFilter(filter) {
        let data = [];
        switch (typeof filter) {
            case "string":
                // data = filter.split('');
                data = [...filter];
                break;
            case "object":
                data = filter;
                break;
            default:
                return;
        }

        setFilterProto(data);
        Context.setItem(WORK_ID.filter, data);
    }

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
        let filters = Array.from(group).map(fmt => {
            for (let i = 0; i < CHAR_GROUP_LIST.length; ++i) {
                if (CHAR_GROUP_LIST[i].value === fmt) {
                    return CHAR_GROUP_LIST[i].filter;
                }
            }
            return () => false
        })
        for (let [name, attrs] of constructTab) {
            if (config && config.correction_table.hasOwnProperty(name)) {
                attrs = config.correction_table[name];
            }
            for (let i = 0; i < filters.length; ++i) {
                if (filters[i](attrs)) {
                    members.push(name)
                }
            }
        }
        if (group.has("Letter")) {
            members.push(...'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz'.split(''))
        }
        if (group.has("Number")) {
            members.push(...'0123456789'.split(''))
        }
        setCharMembers(members);
    }

    function handleScroll(e) {
        if (filter.length === 0) {
            normalOffsetRef.current = e.target.scrollTop;
            Context.setItem(WORK_ID.scrollOffset, e.target.scrollTop);
        }
    }

    let charDatas = filter.length == 0 ? charMembers : filter;
    // Test
    // charDatas = ["巉"]

    charDatas = charDatas.map((char, i) => {
        return {
            id: char + i,
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