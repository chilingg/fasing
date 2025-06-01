import { Space, Flex, Slider, Divider, Button, Descriptions } from 'antd';
import { theme } from 'antd';
const { useToken } = theme;

import { useState, useEffect } from "react";
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

function SliderValue({ label, value, setValue, zeroVal, min = 0, max = 1, step = 0.1 }) {
    const [zero, setZero] = useState(zeroVal);
    const [second, setSecond] = useState((min + max) / 2);

    return <Space size="middle">
        <p>{label}</p>
        <Slider min={min} max={max} step={step} value={value} style={{ width: 80 }} onChange={newVal => setValue(newVal)} />
        <p style={{ width: 16 }}>{value}</p>
        <Button size="small" style={{ width: 32 }} disabled={zero !== zeroVal} onClick={() => {
            setValue(second);
            setSecond(value);
        }}>{second}</Button>
        {zeroVal !== undefined && <Button size="small" style={{ width: 32 }} onClick={() => {
            setValue(zero);
            setZero(value);
        }}>{zero}</Button>}
    </Space>
}

function ConfigSetting({ config, updateConfig }) {
    if (!config) {
        return <></>
    }

    return <Space direction="vertical">
        {

            config.process_control.map((ctrl, i) => {
                let ctrlName;
                let options;
                let data;
                let label;
                for (let attr in ctrl) {
                    ctrlName = attr;
                    data = ctrl[attr];
                }

                switch (ctrlName) {
                    case "Center":
                        label = "视觉重心";
                        options = <>
                            <Flex gap="large" align="center">
                                <p>横轴</p>
                                <Space direction="vertical" size="0">
                                    <SliderValue label="目标" value={data.h.operation} setValue={val => updateConfig(draft => { draft.process_control[i][ctrlName].h.operation = val; })} />
                                    <SliderValue label="执行" value={data.h.execution} setValue={val => updateConfig(draft => { draft.process_control[i][ctrlName].h.execution = val; })} zeroVal={0} />
                                </Space>
                            </Flex>
                            <Flex gap="large" align="center">
                                <p>竖轴</p>
                                <Space direction="vertical" size="0">
                                    <SliderValue label="目标" value={data.v.operation} setValue={val => updateConfig(draft => { draft.process_control[i][ctrlName].v.operation = val; })} />
                                    <SliderValue label="执行" value={data.v.execution} setValue={val => updateConfig(draft => { draft.process_control[i][ctrlName].v.execution = val; })} zeroVal={0} />
                                </Space>
                            </Flex>
                        </>
                        break;
                    case "CompCenter":
                        label = "组合重心";
                        options = <>
                            <Flex gap="large" align="center">
                                <p>横轴</p>
                                <Space direction="vertical" size="0">
                                    <SliderValue label="目标" value={data.h.operation} setValue={val => updateConfig(draft => { draft.process_control[i][ctrlName].h.operation = val; })} />
                                    <SliderValue label="执行" value={data.h.execution} setValue={val => updateConfig(draft => { draft.process_control[i][ctrlName].h.execution = val; })} zeroVal={0} />
                                </Space>
                            </Flex>
                            <Flex gap="large" align="center">
                                <p>竖轴</p>
                                <Space direction="vertical" size="0">
                                    <SliderValue label="目标" value={data.v.operation} setValue={val => updateConfig(draft => { draft.process_control[i][ctrlName].v.operation = val; })} />
                                    <SliderValue label="执行" value={data.v.execution} setValue={val => updateConfig(draft => { draft.process_control[i][ctrlName].v.execution = val; })} zeroVal={0} />
                                </Space>
                            </Flex>
                        </>
                        break;
                    default:
                        label = ctrlName;
                }

                return <div key={i}>
                    <Divider orientation="left" plain>{label}</Divider>
                    {options}
                </div>
            })
        }
    </Space>
}

function Describe({ label, content }) {
    const { token } = useToken();

    return <p><span style={{ color: token.colorTextQuaternary }}>{label}&nbsp;:&nbsp;&nbsp;</span>{content}</p>
}

function CompInfos({ info }) {
    if ('Single' in info) {
        let cinfo = info.Single;
        return <Flex gap="large">
            <p>{cinfo.name}</p>
            <Space direction="vertical" size="0">
                <Describe label={'基础值'} content={`${cinfo.allocs.h.reduce((a, b) => a + b, 0)} * ${cinfo.allocs.v.reduce((a, b) => a + b, 0)}`} />
                <Describe label={'横轴'} content={cinfo.allocs.h.join(", ")} />
                <Describe label={'竖轴'} content={cinfo.allocs.v.join(", ")} />
                <Describe label={'分配值'} content={`${cinfo.assign.h.map(a => a.base + a.excess).reduce((a, b) => a + b, 0).toFixed(3)} * ${cinfo.assign.v.map(a => a.base + a.excess).reduce((a, b) => a + b, 0).toFixed(3)}`} />
                <Describe label={'横轴'} content={cinfo.assign.h.map(a => (a.base + a.excess).toFixed(3)).join(", ")} />
                <Describe label={'竖轴'} content={cinfo.assign.v.map(a => (a.base + a.excess).toFixed(3)).join(", ")} />
                <Describe label={'白边'}
                    content={`左 ${cinfo.offsets.h[0].toFixed(3)}, 左 ${cinfo.offsets.h[1].toFixed(3)}, 前 ${cinfo.offsets.v[0].toFixed(3)}, 后 ${cinfo.offsets.v[1].toFixed(3)}`}
                />
            </Space>
        </Flex>
    } else {
        function dot(d) {
            return d ? '|' : 'x'
        }

        let cinfo = info.Complex;

        let interval = [];
        for (let i = 0; i < cinfo.intervals.length; ++i) {
            interval.push(String(cinfo.intervals_alloc[i]) + "->" + (cinfo.intervals[i].base + cinfo.intervals[i].excess).toFixed(3));
        }

        return <Flex gap="large">
            <p>{cinfo.name}</p>
            <Space direction="vertical" size="0">
                <Describe label={'构成'} content={cinfo.comb_name} />
                <Describe label={'间隔'} content={interval.join(", ")} />
                {cinfo.edges.map((edge, i) => {
                    return <Flex gap="large" key={i}>
                        <p>{i}</p>
                        <Space direction="vertical" size="0">
                            <p>{`${dot(edge[0].dots[0])} ${edge[0].faces[0].toFixed(3)} ${dot(edge[0].dots[1])} ${edge[0].faces[1].toFixed(3)} ${dot(edge[0].dots[2])} ${edge[0].faces[2].toFixed(3)} ${dot(edge[0].dots[3])} ${edge[0].faces[3].toFixed(3)} ${dot(edge[0].dots[4])}`}</p>
                            <p>{`${dot(edge[1].dots[0])} ${edge[1].faces[0].toFixed(3)} ${dot(edge[1].dots[1])} ${edge[1].faces[1].toFixed(3)} ${dot(edge[1].dots[2])} ${edge[1].faces[2].toFixed(3)} ${dot(edge[1].dots[3])} ${edge[1].faces[3].toFixed(3)} ${dot(edge[1].dots[4])}`}</p>
                        </Space>
                    </Flex>
                })}
            </Space>
        </Flex>
    }
}

function CharInfos({ selectedChar }) {
    const [info, setInfo] = useState();

    useEffect(() => {
        if (selectedChar && selectedChar) {
            invoke("get_char_info", { name: selectedChar }).then(info => setInfo(info));
        } else {
            setInfo(undefined);
        }

        let unlistenStrucChange = listen("changed", (e) => {
            if (e.payload == "config" && selectedChar) {
                invoke("get_char_info", { name: selectedChar }).then(info => setInfo(info));
            }
        });

        return () => {
            unlistenStrucChange.then(f => f());
        };
    }, [selectedChar]);

    if (!info) {
        return <></>
    }

    return <Space direction="vertical">
        <p>{info.comb_name}</p>
        <Describe label={'尺寸'} content={`横 ${info.base_size.h} 竖 ${info.base_size.v}`} />
        <Describe label={'重心'} content={`(${info.center[0].toFixed(3)}, ${info.center[1].toFixed(3)})`} />
        <Describe label={'等级'} content={`横 ${info.levels.h} 竖 ${info.levels.v}`} />
        <Describe label={'缩放'} content={`横 ${info.scales.h.toFixed(3)} 竖 ${info.scales.v.toFixed(3)}`} />
        {info.comp_infos.map((comp, i) => <CompInfos key={i} info={comp} />)}
    </Space>
}

const Right = ({ config, updateConfig, selectedChar }) => {
    return <Space direction="vertical" split={<Divider />}>
        <ConfigSetting config={config} updateConfig={updateConfig} />
        <CharInfos selectedChar={selectedChar} />
    </Space>
}

export default Right;