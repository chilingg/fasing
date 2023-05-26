import { Horizontal, Vertical } from "./Line"
import Input from "./Input";

import { useEffect, useRef, useState } from "react"
import style from "@/styles/ColorPicker.module.css"

function genHsbFromRgb(color) {
    if (color.r >= color.g && color.g >= color.b) {

    }
}

function PickerArea() {
    return (
        <div className={style.pickerArea} />
    )
}

function HueSlider({ hue, setHue, horizontal, disabled, ...props }) {
    const pickerRef = useRef();

    let offsetSize;
    let ratio;

    useEffect(() => {
        if (!disabled) {
            let parentSize = { width: pickerRef.current.parentElement.offsetWidth, height: pickerRef.current.parentElement.offsetHeight };
            let size = { width: pickerRef.current.offsetWidth, height: pickerRef.current.offsetHeight };

            if (horizontal) {
                ratio = 100 / parentSize.width;
                offsetSize = size.width / 2 * ratio;
                pickerRef.current.style.left = `${hue * 10 / 36 - offsetSize}%`;
                pickerRef.current.style.top = `-${size.height / parentSize.height * 50 - 50}%`;
                pickerRef.current.style.backgroundColor = `hsl(${Math.max(0, Math.min(359, hue))} 100% 50%)`;
            }
        }
    }, [hue, disabled]);

    return (
        <div style={{ position: "relative", overflow: "visible" }} {...props}>
            <div className={style.hue} horizontal={horizontal} />
            {!disabled && (
                <div
                    ref={pickerRef}
                    className={style.hueSliderPicker}
                    disabled={disabled}
                />
            )}
        </div >
    )
}

export function HuePicker({ hue, setHue, ...props }) {
    return (
        <Horizontal style={{ overflow: "visible" }}>
            <HueSlider horizontal="horizontal" hue={hue} setHue={setHue} disabled={props?.disabled} />
            <Input
                label="Hue"
                value={hue}
                type="number"
                min={0} max={359}
                style={{ width: "4em" }}
                setValue={val => {
                    if (val >= 0 && val < 360) {
                        setHue(val);
                    }
                }}
                {...props}
            />
        </Horizontal>
    )
}

export default function ColorPicker({ }) {
    const [color, setColor] = useState({ r: 255, g: 0, b: 0 });

    return (
        <Horizontal>
            <PickerArea />
            <HueSlider />
            <Vertical>
                <Input label="R" value={color.r} type="number" min={0} max={255} style={{ width: "4em" }}
                    setValue={val => setColor({ ...color, r: val })} />
                <Input label="G" value={color.g} type="number" min={0} max={255} style={{ width: "4em" }}
                    setValue={val => setColor({ ...color, g: val })} />
                <Input label="B" value={color.b} type="number" min={0} max={255} style={{ width: "4em" }}
                    setValue={val => setColor({ ...color, b: val })} />
            </Vertical>
        </Horizontal>
    )
}