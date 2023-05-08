import style from "@/styles/Panel.module.css"

export default function Panel({ children, ...props }) {
    return <div {...props} className={style.panel + (props.className ? ` ${props.className}` : "")}>{children}</div>
}

export function FloatPanel({ children }) {
    return <div className={style.floatPanel}>{children}</div>
}

export function SubPanel({ children, ...props }) {
    return <div {...props} className={style.subpanel + (props.className ? ` ${props.className}` : "")}>{children}</div>
}