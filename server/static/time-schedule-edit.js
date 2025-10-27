function timeScheduleEdit(jsonInputField, domParent) {
    let tableBodyElement = undefined;

    function init() {
        const initialValue = JSON.parse(jsonInputField.value);

        let tableElement = document.createElement("table");
        tableElement.classList.add("table");
        let tableHeadElement = document.createElement("thead");
        tableHeadElement.innerHTML = "<tr><td>von</td><td>bis</td><td>Bezeichnung</td><td></td></tr>";
        tableElement.appendChild(tableHeadElement);
        tableBodyElement = document.createElement("tbody");
        tableElement.appendChild(tableBodyElement);
        domParent.appendChild(tableElement);
        let addButtonDiv = document.createElement("div");
        addButtonDiv.classList.add("float-end");
        let addButton = document.createElement("button");
        addButton.type = "button";
        addButton.classList.add("btn", "btn-sm", "btn-success");
        addButton.innerHTML = '<i class="bi bi-plus-lg"></i> Abschnitt hinzufügen';
        addButton.addEventListener("click", () => {
            let lastRow = tableBodyElement.lastElementChild;
            addRow();
            updateRowAfterNextRowAdd(lastRow);
            updateJsonResult();
        });
        addButtonDiv.appendChild(addButton);
        domParent.appendChild(addButtonDiv);

        for (const section of initialValue.sections) {
            addRow(section.end_time, section.name);
        }
        let rows = Array.from(tableBodyElement.children);
        for (let [i, row] of rows.entries()) {
            if (i === rows.length - 1) {
                updateRowAfterNextRowRemove(row);
            } else {
                updateRowAfterNextRowAdd(row);
            }
        }
    }

    function addRow(endTime, name) {
        let rowElement = document.createElement("tr");
        rowElement.innerHTML = '<td class="align-middle text-end"><span class="section-beginTimeLabel">…</span>&nbsp;–</td>\
                                <td class="align-middle">\
                                    <input type="time" step="1" class="section-endInput form-control form-control-sm d-none" aria-label="End-Zeit des Abschnitts" />\
                                    <div class="section-lastEndLabel">Tagesende</div>\
                                </td>\
                                <td><input type="text" class="section-nameInput form-control form-control-sm" aria-label="Bezeichnung des Abschnitts"/></td>\
                                <td>\
                                    <button type="button" class="section-removeButton btn btn-sm btn-danger" title="Abschnitt entfernen">\
                                        <i class="bi bi-dash-lg"></i>\
                                    </button>\
                                </td>';
        rowElement.getElementsByClassName("section-removeButton")[0]
            .addEventListener("click", () => {
                removeRow(rowElement);
            });
        let endInput = rowElement.getElementsByClassName("section-endInput")[0];
        let nameInput = rowElement.getElementsByClassName("section-nameInput")[0]
        endInput.addEventListener("change", () => {
            let nextRow = rowElement.nextElementSibling;
            if (nextRow) {
                updateRowAfterPrevRowChange(nextRow);
            }
            updateJsonResult();
        });
        nameInput.addEventListener("change", () => { updateJsonResult(); });

        if (endTime) {
            endInput.value = endTime;
        }
        if (name) {
            nameInput.value = name;
        }
        tableBodyElement.appendChild(rowElement);
        updateRowAfterPrevRowChange(rowElement);
    }

    function removeRow(rowElement) {
        let prevRow = rowElement.previousElementSibling;
        let nextRow = rowElement.nextElementSibling;
        rowElement.remove();
        if (prevRow) {
            updateRowAfterNextRowRemove(prevRow);
        }
        if (nextRow) {
            updateRowAfterPrevRowChange(nextRow);
        }
        updateJsonResult();
    }

    function updateRowAfterPrevRowChange(rowElement) {
        let prevRow = rowElement.previousElementSibling;
        let beginTimeLabel = rowElement.getElementsByClassName("section-beginTimeLabel")[0];
        if (prevRow) {
            let prevEndInput = prevRow.getElementsByClassName("section-endInput")[0];
            beginTimeLabel.innerText = prevEndInput.value;
        } else {
            beginTimeLabel.innerText = "…";
        }
    }

    function updateRowAfterNextRowRemove(rowElement) {
        let isLastRow = !rowElement.nextElementSibling;
        let isOnlyRow = !rowElement.nextElementSibling && !rowElement.previousElementSibling;
        if (isLastRow) {
            rowElement.getElementsByClassName("section-endInput")[0].classList.add("d-none");
            rowElement.getElementsByClassName("section-lastEndLabel")[0].classList.remove("d-none");
        }
        if (isOnlyRow) {
            rowElement.getElementsByClassName("section-removeButton")[0].disabled = true;
        }
    }

    function updateRowAfterNextRowAdd(rowElement) {
        rowElement.getElementsByClassName("section-endInput")[0].classList.remove("d-none");
        rowElement.getElementsByClassName("section-lastEndLabel")[0].classList.add("d-none");
        rowElement.getElementsByClassName("section-removeButton")[0].disabled = false;
    }

    function updateJsonResult() {
        const nameInputs = tableBodyElement.getElementsByClassName("section-nameInput");
        const endInputs = tableBodyElement.getElementsByClassName("section-endInput");
        let sections = Array.from(nameInputs)
            .map((input) => input.value)
            .map((name, i) => {
                return {
                    "name": name,
                    "end_time": (i === nameInputs.length - 1) ? null : endInputs[i].value
                };
            });
        jsonInputField.value = JSON.stringify({"sections": sections});
    }

    init();
}
