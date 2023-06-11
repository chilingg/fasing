import { CloseIcon } from "./Icons";
import { useState, useRef } from "react";
import style from "@/styles/Input.module.css";

export default function Input({ type = "text", label, value, setValue, extension, ...props }) {
    const [focused, setFocused] = useState(false);
    const inputRef = useRef();

    return (
        <>
            {label && <label>{label}</label>}
            <div className={style.inputContainer} extension={extension ? "extension" : undefined}>
                <input
                    ref={inputRef}
                    className={style.input}
                    type={type}
                    value={value}
                    onChange={(e) => setValue && setValue(e.target.value)}
                    onFocus={() => setFocused(true)}
                    onBlur={() => setFocused(false)}
                    {...props}
                />
                {type === "text" && focused && inputRef.current.value && (
                    <div className={style.clearBtn} onMouseDown={e => e.preventDefault()} onClick={() => {
                        setValue && setValue("");
                        inputRef.current.value = "";
                    }}>
                        <CloseIcon size={10} pos={1} />
                    </div>
                )}
            </div >
        </>
    )
}
