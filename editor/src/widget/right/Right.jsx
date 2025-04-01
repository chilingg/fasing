import { Space, Flex, Slider, Divider, Button, Descriptions } from 'antd';
import { theme } from 'antd';
const { useToken } = theme;

import { useState, useEffect } from "react";
import { invoke } from '@tauri-apps/api/core';

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
    const [test, setTest] = useState(0.2);

    if (!config) {
        return <></>
    }

    return <Space direction="vertical">
        <Divider orientation="left" plain>
            视觉重心
        </Divider>
        <Flex gap="large" align="center">
            <p>横轴</p>
            <Space direction="vertical" size="0">
                <SliderValue label="目标" value={config.center.h.operation} setValue={val => updateConfig(draft => { draft.center.h.operation = val; })} />
                <SliderValue label="执行" value={config.center.h.execution} setValue={val => updateConfig(draft => { draft.center.h.execution = val; })} zeroVal={0} />
            </Space>
        </Flex>
        <Flex gap="large" align="center">
            <p>竖轴</p>
            <Space direction="vertical" size="0">
                <SliderValue label="目标" value={config.center.v.operation} setValue={val => updateConfig(draft => { draft.center.v.operation = val; })} />
                <SliderValue label="执行" value={config.center.v.execution} setValue={val => updateConfig(draft => { draft.center.v.execution = val; })} zeroVal={0} />
            </Space>
        </Flex>


        <Divider orientation="left" plain>
            组合重心
        </Divider>
        <Flex gap="large" align="center">
            <p>横轴</p>
            <Space direction="vertical" size="0">
                <SliderValue label="目标" value={config.comp_center.h.operation} setValue={val => updateConfig(draft => draft.comp_center.h.operation = val)} />
                <SliderValue label="执行" value={config.comp_center.h.execution} setValue={val => updateConfig(draft => draft.comp_center.h.execution = val)} zeroVal={0} />
            </Space>
        </Flex>
        <Flex gap="large" align="center">
            <p>竖轴</p>
            <Space direction="vertical" size="0">
                <SliderValue label="目标" value={config.comp_center.v.operation} setValue={val => updateConfig(draft => draft.comp_center.v.operation = val)} />
                <SliderValue label="执行" value={config.comp_center.v.execution} setValue={val => updateConfig(draft => draft.comp_center.v.execution = val)} zeroVal={0} />
            </Space>
        </Flex>
    </Space>
}

function Describe({ label, content }) {
    const { token } = useToken();

    return <p><span style={{ color: token.colorTextTertiary }}>{label}&nbsp;:&nbsp;&nbsp;</span>{content}</p>
}

function CompInfos({ info }) {
    let children;
    let cinfo;

    if ('Single' in info) {
        cinfo = info.Single;
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
        cinfo = info.Complex;
        return <p>{cinfo.name}</p>
    }
}

function CharInfos({ selectedChar }) {
    const [info, setInfo] = useState();

    useEffect(() => {
        if (selectedChar) {
            invoke("get_char_info", { name: selectedChar }).then(info => setInfo(info));
        } else {
            setInfo(undefined);
        }
    }, [selectedChar]);

    if (!info) {
        return <></>
    }

    return <Space direction="vertical">
        <Divider orientation="left" plain>
            {info.comb_name}
        </Divider>
        <Describe label={'重心'} content={`(${info.center[0].toFixed(3)}, ${info.center[1].toFixed(3)})`} />
        <Describe label={'等级'} content={`横 ${info.levels.h} 竖 ${info.levels.v}`} />
        <Describe label={'缩放'} content={`横 ${info.scales.h.toFixed(3)} 竖 ${info.scales.v.toFixed(3)}`} />
        <Describe label={'白边'}
            content={`左 ${info.white_areas.h[0].toFixed(3)}, 左 ${info.white_areas.h[1].toFixed(3)}, 前 ${info.white_areas.v[0].toFixed(3)}, 后 ${info.white_areas.v[1].toFixed(3)}`}
        />
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