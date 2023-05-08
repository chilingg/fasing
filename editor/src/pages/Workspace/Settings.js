import Panel from "@/widgets/Panel";
import style from "@/styles/Settings.module.css"

export default function Settings({ children }) {
    return <Panel className={style.settings}>{children}</Panel>
}
