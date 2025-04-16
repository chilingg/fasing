import { CHAR_GROUP_LIST } from '../../lib/construct';

import { Input, InputNumber, Space, Divider, ColorPicker, Switch, Checkbox, Button } from 'antd';
const { TextArea } = Input;

const FONT_SIZE_RANGE = [8, 128]
const DIsplaySettings = ({ charDisplay, setCharDisplay, strokWidth }) => {
    function handleSizeChange(e) {
        let value = Math.min(Math.max(e.target.value, FONT_SIZE_RANGE[0]), FONT_SIZE_RANGE[1]);
        setCharDisplay({ ...charDisplay, size: value });
    }

    return <Space direction='vertical' size={'middle'}>
        <Space size={'middle'}>
            <div>字色：<ColorPicker
                defaultValue={charDisplay.color} size="small" disabledAlpha
                onChangeComplete={(color) => setCharDisplay({ ...charDisplay, color: color.toCssString() })}
            /></div>
            <div>背景：<ColorPicker
                defaultValue={charDisplay.background} size="small" disabledAlpha
                onChangeComplete={(color) => setCharDisplay({ ...charDisplay, background: color.toCssString() })}
            /></div>
            <div>字号：<InputNumber
                min={FONT_SIZE_RANGE[0]}
                max={FONT_SIZE_RANGE[1]}
                size='small'
                defaultValue={charDisplay.size}
                style={{ width: 32 }}
                onBlur={handleSizeChange}
                onPressEnter={handleSizeChange}
            /></div>
            <div>字名：<Switch
                size='small'
                defaultValue={charDisplay.charName}
                onChange={checked => setCharDisplay({ ...charDisplay, charName: checked })}
            />
            </div>
        </Space>
        <div>线宽：{strokWidth} | {Math.round(charDisplay.size * strokWidth)} px</div>
    </Space>
}

const Filters = ({ charFilter, setCharFilter }) => {
    function handleChange(e) {
        setCharFilter({ ...charFilter, text: e.target.value })
    }

    return <Space direction="vertical" size={'middle'} style={{ width: "100%" }}>
        <TextArea
            showCount
            maxLength={1000}
            defaultValue={charFilter.text}
            style={{
                height: 120,
                resize: 'none',
            }}
            onBlur={handleChange}
        />
        <br />
        <Checkbox.Group options={CHAR_GROUP_LIST} defaultValue={charFilter.types} onChange={list => setCharFilter({ ...charFilter, types: list })} />
    </Space>
}

const Left = ({ charDisplay, setCharDisplay, charFilter, setCharFilter, strokWidth }) => {
    return <Space
        direction="vertical" split={<Divider />}
    >
        <DIsplaySettings charDisplay={charDisplay} setCharDisplay={setCharDisplay} strokWidth={strokWidth} />
        <Filters charFilter={charFilter} setCharFilter={setCharFilter} />
    </Space>
}

export default Left;