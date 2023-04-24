import style from "@/styles/Panel.module.css"

export default function Panel({ children }) {
    return <div className={style.panel}>{children}</div>
}

export function FloatPanel({ children }) {
    return <div className={style.floatPanel}>{children}</div>
}