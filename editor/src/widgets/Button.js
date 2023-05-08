import style from '@/styles/Button.module.css'
import Menu from './Menu';

export function MenuBtn({ text, menuItems }) {
    let className = `${style.btn} ${style.menuBtn}`;
    return (
        <>
            <button className={className}>
                {text}
            </button>
            <Menu items={menuItems}></Menu>
        </>
    )
}

export function ActionBtn({ children, active, value, onAction }) {
    let className = `${style.btn} ${style.activeBtn}`;
    return (
        <button className={className} active={active ? "" : undefined} value={value} onClick={e => onAction(e, !active, value)}>{children}</button>
    )
}

export function IconBtn({ children, btnStyle, onClick, active }) {
    let attr = {};
    attr.className = `${style.btn} ${style.iconBtn}`;
    if (btnStyle) {
        attr.style = btnStyle;
    }
    if (active) {
        attr.active = "";
    }

    return (
        <button {...attr} onClick={onClick}>{children}</button>
    );
}