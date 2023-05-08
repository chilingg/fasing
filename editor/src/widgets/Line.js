import style from "@/styles/Line.module.css"

export function Horizontal({ children, ...props }) {
    return <div className={style.horizontal} {...props}>{children}</div>
}

export function Vertical({ children, ...props }) {
    return <div className={style.vertical} {...props}>{children}</div>
}
