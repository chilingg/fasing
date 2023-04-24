import Footer from "./Footer"
import Panel from "../Panel"

function Settings() {
    return <Panel>123</Panel>
}

function Main() {
    return <div style={{ display: "flex", flex: 1 }}></div>
}

export default function Workspace() {
    return (
        <div style={{ display: "flex", flex: 1, flexDirection: "column" }}>
            <Settings></Settings>
            <Main></Main>
            <Footer></Footer>
        </div>
    )
}