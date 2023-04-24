import style from '@/styles/Button.module.css'
import Menu from './Menu';

export function MenuBtn({ text, menuItems, running }) {
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