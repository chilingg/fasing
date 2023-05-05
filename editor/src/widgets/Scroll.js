import { Item } from "./List";
import { useEffect, useRef } from "react";

import style from "@/styles/Scroll.module.css"

function ScrollBar({ offset = 0 }) {
    const scrollbarRef = useRef();

    useEffect(() => {
        let scrollbar = scrollbarRef.current;
        let scrollArea = scrollbar.parentElement.childNodes[0];

        let scrollLength = scrollArea.scrollHeight;
        let clientLength = scrollArea.clientHeight;
        if (scrollLength > clientLength) {
            let ratio = (clientLength / scrollLength);
            let height = clientLength * ratio;
            let top = offset * ratio;

            scrollbar.style.visibility = "visible";
            scrollbar.style.height = height + "px";
            scrollbar.style.top = String(top) + "px";

            scrollArea.addEventListener("scroll", () => {
                scrollbar.style.top = String(scrollArea.scrollTop * ratio) + "px";
            });
        } else {
            scrollbar.style.visibility = "hidden";
        }
    });

    function handleDrag(e) {
        let scrollbar = scrollbarRef.current;
        let scrollArea = scrollbar.parentElement.childNodes[0];

        let scrollLength = scrollArea.scrollHeight;
        let clientLength = scrollArea.clientHeight;
        let ratio = (clientLength / scrollLength);

        let maxOffset = (scrollLength - clientLength) * ratio;

        let current = parseInt(scrollbar.style.top);
        current = current ? current : 0;
        let next = Math.max(0, Math.min(maxOffset, e.movementY + current));

        scrollArea.scrollBy(0, (next - current) / ratio)

        e.preventDefault();
    }

    function handleDragStart(e) {
        if (e.button === 0) {
            window.addEventListener("mousemove", handleDrag);
            window.addEventListener("mouseup", handleDragEnd);
        }
    }

    function handleDragEnd() {
        window.removeEventListener("mousemove", handleDrag);
        window.removeEventListener("mouseup", handleDragEnd);
    }

    return <div ref={scrollbarRef} className={style.scrollbar} onMouseDown={handleDragStart} />
}

export function ItemsScrollArea({ items, ItemType, offset = 0, onScroll }) {
    const areaRef = useRef();
    const scrollValue = useRef(0);

    useEffect(() => areaRef.current.scrollBy(0, offset), [offset]);

    function smoothScroll() {
        if (scrollValue.current !== 0) {
            let delta = (scrollValue.current > 0 ? 1 : -1) * 10;
            let value = Math.abs(delta) < Math.abs(scrollValue.current) ? delta : scrollValue.current;
            areaRef.current.scrollBy(0, value);
            scrollValue.current -= value;
            setTimeout(smoothScroll);
        }
    }

    function handleScroll(e) {
        let delta = e.deltaY > 0 ? 1 : -1;

        if (scrollValue.current === 0) {
            scrollValue.current += delta * 100;
            smoothScroll()
        } else {
            scrollValue.current += delta * 100;
        }
    }

    return (
        <div className={style.area}>
            <ul ref={areaRef} onWheel={handleScroll} onScroll={onScroll}>
                {items.map(item => (
                    <Item key={item.id}>
                        <ItemType {...item.data} />
                    </Item>
                ))}
            </ul>
            <ScrollBar offset={offset} />
        </div>
    )
}