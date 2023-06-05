import { useState, useRef } from "react"

const DIRECTION_TYPES = {
    "r": { cursor: "e-resize", vec: { x: 1, y: 0 } },
    "rt": { cursor: "ne-resize", vec: { x: 1, y: -1 } },
    "lt": { cursor: "nw-resize", vec: { x: -1, y: -1 } },
    "t": { cursor: "n-resize", vec: { x: 0, y: -1 } },
    "rb": { cursor: "se-resize", vec: { x: 1, y: 1 } },
    "lb": { cursor: "sw-resize", vec: { x: -1, y: 1 } },
    "b": { cursor: "s-resize", vec: { x: 0, y: 1 } },
    "l": { cursor: "w-resize", vec: { x: -1, y: 0 } },
    "": { cursor: "" },
}

export default function ResizableArea({
    children,
    width,
    height,
    left,
    right,
    top,
    bottom,
    onResize,
    style,
    minWidth = 0,
    minHeight = 0,
    maxWidth = 99999,
    MaxHeight = 99999,
    ...props
}) {
    // const [resize, setResize] = useState(null);
    const areaRef = useRef();
    const directionRef = useRef("");

    const edge = 5;
    let resize_vec = null;
    let resize = null;

    function directionCheck(pos, rect) {
        let inLeft = left && (Math.abs(pos.x - rect.left) < edge);
        let inRight = right && (Math.abs(rect.right - pos.x) < edge);
        let inTop = top && (Math.abs(pos.y - rect.top) < edge);
        let inBottom = bottom && (Math.abs(rect.bottom - pos.y) < edge);

        if (inLeft && inTop) {
            return "lt";
        } else if (inTop && inRight) {
            return "rt";
        } else if (inRight && inBottom) {
            return "rb";
        } else if (inBottom && inLeft) {
            return "lb";
        } else if (inLeft) {
            return "l";
        } else if (inTop) {
            return "t";
        } else if (inRight) {
            return "r";
        } else if (inBottom) {
            return "b";
        } else {
            return "";
        }
    }

    function handleMouseMove(e) {
        let rect = areaRef.current.getBoundingClientRect();

        let direction = directionCheck({ x: e.clientX, y: e.clientY }, rect);
        if (direction !== directionRef.current) {
            areaRef.current.style.cursor = DIRECTION_TYPES[direction].cursor;
            directionRef.current = direction;
        }
    }

    function handleMouseUpInDoc() {
        document.removeEventListener("mouseup", handleMouseUpInDoc);
        document.removeEventListener("mousemove", handleMouseMoveInDoc);
    }

    function handleMouseMoveInDoc(e) {
        let rect = areaRef.current.getBoundingClientRect();
        if (resize_vec.x) {
            let width = Math.max(minWidth, Math.min(maxWidth, (e.clientX - resize.x) * resize_vec.x + resize.width));
            areaRef.current.style.width = String(width) + "px";
        }
        if (resize_vec.y) {
            let height = Math.max(minHeight, Math.min(MaxHeight, (e.clientY - resize.y) * resize_vec.y + resize.height));
            areaRef.current.style.height = String(height) + "px";
        }
        onResize && onResize(rect);
    }

    function handleMouseDowne(e) {
        if (directionRef.current) {
            resize_vec = DIRECTION_TYPES[directionRef.current].vec;
            resize = { x: e.clientX, y: e.clientY, width: parseInt(areaRef.current.clientWidth), height: parseInt(areaRef.current.clientHeight) };
            document.addEventListener("mouseup", handleMouseUpInDoc);
            document.addEventListener("mousemove", handleMouseMoveInDoc);

            e.preventDefault();
        }
    }

    let compStyle = style ? { ...style } : {};
    if (width || height) {
        if (width) {
            compStyle.width = width + 'px';
        }
        if (height)
            compStyle.height = height + 'px';
    }

    return (
        <div
            ref={areaRef}
            style={compStyle}
            onMouseMove={handleMouseMove}
            onMouseDown={handleMouseDowne}
            onMouseLeave={() => areaRef.current.style.cursor = ""}
            {...props}
        >
            {children}
        </div>
    );
}