import style from "@/styles/Line.module.css"

export function Horizontal({ children, spacing = true, ...props }) {
    return <div className={style.horizontal} spacing={spacing ? "spacing" : undefined} {...props}>{children}</div>
}

export function Vertical({ children, spacing = true, ...props }) {
    return <div className={style.vertical} spacing={spacing ? "spacing" : undefined} {...props}>{children}</div>
}
