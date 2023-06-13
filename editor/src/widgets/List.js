import style from "@/styles/List.module.css";

export function List({ direction = "row", children, ...props }) {
    let listStyle = props.style ? props.style : {};

    return <ul style={{ ...listStyle, display: "flex", flexDirection: direction }} {...props}>{children}</ul>;
}

export function Item({ children, ...props }) {
    let itemStyle = props.style ? props.style : {};

    return <li style={{ listStyleType: "none", ...itemStyle }} {...props}>{children}</li>
}

export function LabelList({ items }) {
    return (
        <ul className={style.labelList}>
            {items.map(item => <li className={style.labelListItem} key={item.id}>{item.label}</li>)}
        </ul>
    )
}