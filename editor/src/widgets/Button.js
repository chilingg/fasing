import style from '@/styles/Button.module.css'
import Menu from './Menu';

export function Button({ children, onClick }) {
    return (
        <button className={style.button} onClick={onClick}>{children}</button>
    )
}

export function MenuBtn({ text, menuItems }) {
    return (
        <>
            <button className={style.menuBtn}>
                {text}
            </button>
            <Menu items={menuItems}></Menu>
        </>
    )
}

export function ActionBtn({ children, active, value, onAction }) {
    return (
        <button className={style.button} active={active ? "" : undefined} value={value} onClick={e => onAction(e, !active, value)}>{children}</button>
    )
}

export function IconBtn({ children, btnStyle, onClick, active }) {
    let attr = {};
    if (btnStyle) {
        attr.style = btnStyle;
    }
    if (active) {
        attr.active = "";
    }

    return (
        <button className={style.iconBtn} {...attr} onClick={onClick}>{children}</button>
    );
}