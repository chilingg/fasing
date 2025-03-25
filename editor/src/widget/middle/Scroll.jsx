import { useEffect, useRef, useState } from "react";

import { theme } from 'antd';
const { useToken } = theme;

function ScrollBar() {
    const scrollbarRef = useRef();
    const token = useToken();

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

    const style = {
        width: '10px',
        backgroundColor: token.token.colorBgSolid,
        position: 'absolute',
        right: '0',
        top: '0',
    }

    return <div ref={scrollbarRef} style={style} onMouseDown={handleDragStart} />
}

export default function ItemsScrollArea({ items, initOffset = 0, updateArea = undefined, onScroll = undefined, ...props }) {
    const areaRef = useRef();
    const listRef = useRef();
    const offset = useRef(0);
    const itemSize = useRef();
    const scrollValue = useRef(0);

    const [padding, setPadding] = useState(0);
    const [itemRange, setItemRange] = useState([0, 1]);

    useEffect(() => {
        let itemRectRef = areaRef.current.querySelector("li")?.getBoundingClientRect();
        if (!itemRectRef) {
            return;
        } else {
            if (itemSize.current) {
                if (itemSize.current.width == itemRectRef.width && itemSize.current.height == itemRectRef.height) {
                    return
                }
            }
            itemSize.current = itemRectRef;
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
    }, [padding, initOffset]);

    function smoothScroll() {
        if (scrollValue.current !== 0) {
            let delta = (scrollValue.current > 0 ? 1 : -1) * 5;
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
        onScroll && onScroll(e);
    }

    function handleWheel(e) {
        let delta = e.deltaY > 0 ? 1 : -1;

        if (scrollValue.current === 0) {
            scrollValue.current += delta * 40;
            smoothScroll()
        } else {
            scrollValue.current += delta * 40;
        }
    }

    let itemHeigh = itemSize.current?.height || 1;
    const styles = {
        area: {
            width: '100%',
            height: '100%',
            overflow: 'hidden',
            position: 'relative'
        },
        ul: {
            display: 'flex',
            flexWrap: 'wrap',
            transform: `translateY(${Math.floor(offset.current / itemHeigh) * itemHeigh - padding}px)`,
            overflow: "visible",
            listStyleType: 'none',
            margin: 0,
            padding: 0,
        },
    };

    return (
        <div style={styles.area} ref={areaRef} onWheel={handleWheel} onScroll={handleScroll} {...props}>
            <div style={{ height: padding, width: '100%' }}></div>
            <ul ref={listRef} style={styles.ul}>
                {items.slice(itemRange[0], itemRange[1]).map(item => (
                    <li key={item.id}>
                        {item.data}
                    </li>
                ))}
            </ul>
            <ScrollBar />
        </div >
    )
}