import { ActionBtn } from "./Button"
import { Horizontal } from "./Line"

export function SelectionLabel({ items, currents, onChange }) {
    return (
        <Horizontal>{
            items.map(item =>
                <ActionBtn key={item.value} active={currents.has(item.value)} value={item.value} onAction={onChange}>{item.label}</ActionBtn>
            )
        }</Horizontal>
    )
}