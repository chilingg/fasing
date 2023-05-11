import ComponentsWorkspace from "./ComponentWorkspace";

export default function Workspace({ workStage }) {
    let current = <div></div>;

    switch (workStage) {
        case "components":
            current = <ComponentsWorkspace />;
            break;
    }

    return (
        <div style={{ flex: 1 }}>
            {current}
        </div>
    )
}