import style from "@/styles/List.module.css";

export function List({ direction = "row", children }) {
    return <ul style={{ display: "flex", flexDirection: direction }}>{children}</ul>;
}

export function Item({ children }) {
    return <li style={{ listStyleType: "none" }}>{children}</li>
}

export function LabelList({ items }) {
    return (
        <ul className={style.labelList}>
            {items.map(item => <li className={style.labelListItem} key={item.id}>{item.label}</li>)}
        </ul>
    )
}