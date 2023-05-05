export function List({ direction = "row", children }) {
    return <ul style={{ display: "flex", flexDirection: direction }}>{children}</ul>;
}

export function Item({ children }) {
    return <li style={{ listStyleType: "none" }}>{children}</li>
}
