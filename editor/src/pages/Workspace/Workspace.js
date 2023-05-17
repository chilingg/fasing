import ComponentsWorkspace from "./ComponentWorkspace";
import CombinationWOrkspace from "./CombinationWorkspace";

export default function Workspace({ workStage }) {
    let current = <div></div>;

    switch (workStage) {
        case "components":
            current = <ComponentsWorkspace />;
            break;
        case "combination":
            current = <CombinationWOrkspace />;
            break;
    }

    return (
        <div style={{ flex: 1 }}>
            {current}
        </div>
    )
}