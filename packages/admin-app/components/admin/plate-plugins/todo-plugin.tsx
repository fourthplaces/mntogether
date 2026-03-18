"use client";

import React, { useCallback } from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";
import type { TElement } from "platejs";
import { Square, SquareCheck } from "lucide-react";

export const TODO_KEY = "todo";

export function TodoElement(props: PlateElementProps) {
  const { children, editor, element, ...rest } = props;
  const data = element as TElement & { checked?: boolean };

  const toggleChecked = useCallback(() => {
    const path = editor.api.findPath(element);
    if (path) {
      editor.tf.setNodes({ checked: !data.checked } as Partial<TElement>, { at: path });
    }
  }, [editor, element, data.checked]);

  return (
    <PlateElement {...rest} element={element} editor={editor} className="todo-item">
      <span {...{contentEditable: false} as any} className="todo-item__checkbox" onMouseDown={(e) => { e.preventDefault(); toggleChecked(); }}>
        {data.checked ? <SquareCheck size={16} strokeWidth={2} /> : <Square size={16} strokeWidth={2} />}
      </span>
      <span className={`todo-item__text ${data.checked ? "todo-item__text--checked" : ""}`}>
        {children}
      </span>
    </PlateElement>
  );
}

export const TodoPlugin = createPlatePlugin({
  key: TODO_KEY,
  node: { isElement: true, type: TODO_KEY },
  render: { node: TodoElement },
});
