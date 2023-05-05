
import Panel from "@/widgets/Panel"

export default function Footer({ children }) {
    return (
        <Panel>
            <footer style={{ height: "2.2em", display: 'flex', alignItems: "center", padding: "0 0.8em" }}>
                {children}
            </footer>
        </Panel>
    )
}
