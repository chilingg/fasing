export function GreaterThanIcon({ className, style, size = 12, ...props }) {
    return (
        <svg className={className} style={style} {...props}>
            <polyline points={`${size * .3},${size * .16} ${size * .6},${size * .5} ${size * .3},${size * .8} `}></polyline>
        </svg>
    )
}

export function CloseIcon({ className, style, size = 12, pos = 0, ...props }) {
    return (
        <svg className={className} style={style} {...props}>
            <line x1={2 + pos} y1={2 + pos} x2={size - 2 + pos} y2={size - 2 + pos}></line>
            <line x1={size - 2 + pos} y1={2 + pos} x2={2 + pos} y2={size - 2 + pos}></line>
        </svg>
    )
}