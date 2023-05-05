import style from "@/styles/StrucDisplay.module.css";

export default function StrucDisplay({ name }) {
    return (
        <div className={style.area}>
            <svg className={style.canvas}></svg>
            <p>{name}</p>
        </div>
    )
}
