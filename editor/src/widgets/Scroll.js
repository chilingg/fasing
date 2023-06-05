import { Item } from "./List";
import { useEffect, useRef, useState } from "react";

import style from "@/styles/Scroll.module.css"

function ScrollBar() {
    const scrollbarRef = useRef();

    useEffect(() => {
        let scrollbar = scrollbarRef.current;
        let scrollArea = scrollbar.parentElement;

        let scrollLength = scrollArea.scrollHeight;
        let clientLength = scrollArea.clientHeight;
        if (scrollLength > clientLength) {
            let ratio = (clientLength / scrollLength);
            let height = clientLength * ratio;
            let top = scrollArea.scrollTop * (ratio + 1);

            scrollbar.style.visibility = "visible";
            scrollbar.style.height = height + "px";
            scrollbar.style.top = String(top) + "px";

            function handelScroll() {
                scrollbar.style.top = String(scrollArea.scrollTop * (ratio + 1)) + "px";
            }

            scrollArea.addEventListener("scroll", handelScroll);
            return () => scrollArea.removeEventListener("scroll", handelScroll);
        } else {
            scrollbar.style.visibility = "hidden";
        }
    });

    function handleDrag(e) {
        let scrollbar = scrollbarRef.current;
        let scrollArea = scrollbar.parentElement;

        let scrollLength = scrollArea.scrollHeight;
        let clientLength = scrollArea.clientHeight;
        let ratio = (clientLength / scrollLength);

        let maxOffset = scrollLength - clientLength;

        let current = scrollArea.scrollTop;
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

export function ItemsScrollArea({ items, ItemType, initOffset = 0, onScroll }) {
    const areaRef = useRef();
    const listRef = useRef();
    const offset = useRef(0);
    const itemSize = useRef();
    const scrollValue = useRef(0);

    const [padding, setPadding] = useState(0);
    const [itemRange, setItemRange] = useState([0, 50]);

    useEffect(() => {
        if (!itemSize.current) {
            let itemRect = areaRef.current.querySelector("li")?.getBoundingClientRect();
            if (!itemRect) {
                return;
            } else {
                itemSize.current = itemRect;
            }
        }
        let itemRect = itemSize.current;
        let rect = areaRef.current.getBoundingClientRect();
        let row = Math.floor(rect.width / itemRect.width);
        let col = Math.ceil(rect.height / itemRect.height) + 1;
        let height = Math.ceil(items.length / row) * itemRect.height;

        if (height > col * itemRect.height) {
            setPadding(height - col * itemRect.height);
        } else {
            setPadding(0);
        }

        updateItemRange(areaRef.current.scrollTop);
    }, [items]);

    useEffect(() => {
        offset.current = initOffset;
        areaRef.current.scrollTo(0, initOffset);
    }, [padding]);

    function smoothScroll() {
        if (scrollValue.current !== 0) {
            let delta = (scrollValue.current > 0 ? 1 : -1) * 10;
            let value = Math.abs(delta) < Math.abs(scrollValue.current) ? delta : scrollValue.current;
            areaRef.current.scrollBy(0, value);
            scrollValue.current -= value;
            setTimeout(smoothScroll);
        }
    }

    function updateItemRange(offset) {
        let itemRect = itemSize.current;
        let rect = areaRef.current.getBoundingClientRect();
        let row = Math.floor(rect.width / itemRect.width);
        let col = Math.ceil(rect.height / itemRect.height) + 1;
        let start = Math.floor(offset / itemRect.height) * row;

        setItemRange([start, start + row * col]);
    }

    function handleScroll(e) {
        offset.current = e.target.scrollTop;
        updateItemRange(offset.current);
        onScroll(e);
    }

    function handleWheel(e) {
        let delta = e.deltaY > 0 ? 1 : -1;

        if (scrollValue.current === 0) {
            scrollValue.current += delta * 100;
            smoothScroll()
        } else {
            scrollValue.current += delta * 100;
        }
    }

    let itemHeigh = itemSize.current?.height || 1;
    return (
        <div className={style.area} ref={areaRef} onWheel={handleWheel} onScroll={handleScroll}>
            <div className={style.padding} style={{ height: padding }}></div>
            <ul ref={listRef} style={{ transform: `translateY(${Math.floor(offset.current / itemHeigh) * itemHeigh - padding}px)` }}>
                {items.slice(itemRange[0], itemRange[1]).map(item => (
                    <Item key={item.id}>
                        <ItemType {...item.data} />
                    </Item>
                ))}
            </ul>
            <ScrollBar />
        </div>
    )
}