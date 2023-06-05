import { ActionBtn, SelectBtn } from "./Button"
import { Horizontal, Vertical } from "./Line"

export function SelectionLabel({ items, currents, onChange, vertical = false }) {
    let Direction = vertical ? Vertical : Horizontal;

    return (
        <Direction>{
            items.map(item =>
                <ActionBtn key={item.value} active={currents.has(item.value)} value={item.value} onAction={onChange}>{item.label}</ActionBtn>
            )
        }</Direction>
    )
}

export function RadioLabel({ items, currents, onChange, vertical = false }) {
    let Direction = vertical ? Vertical : Horizontal;

    return (
        <Direction>{
            items.map(item =>
                <ActionBtn key={item.value} active={currents === item.value} value={item.value} onAction={(e, active, value) => {
                    if (active) {
                        onChange(e, value);
                    }
                }}>{item.label}</ActionBtn>
            )
        }</Direction>
    )
}

export function Selections({ items, currents, onChange }) {
    return (
        <>{
            items.map(item =>
            (
                <div key={item.value}>
                    <SelectBtn checked={currents.has(item.value)} value={item.value} onClick={onChange} />
                    <label style={{ marginLeft: "4px" }}>{item.label}</label>
                </div>
            )
            )
        }</>
    )
}