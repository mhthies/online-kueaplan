function readDateSelect(daySelect) {
    const date = new Date(daySelect.value);
    return isNaN(date) ? null : date;
}

function readNaiveTimeInput(beginInput) {
    // All times are local time/naive/without timezone, but JavaScript does not support this concept.
    // So, we act like all times are UTC to avoid any conversion.
    const time = new Date("1970-01-01T" + beginInput.value + "Z");
    return isNaN(time) ? null : time;
}

function readNiceDurationInput(durationInput) {
    return parseNiceDurationHours(durationInput.value);
}

function parseNiceDurationHours(value) {
    // The parsing code corresponds to NiceDurationHours::from_form_value() in the Rust code
    const regex = /^(?:(?<d>\d+)d\s*)?(?<H>\d+(?:[.,]\d{1,7})?)(?::(?<M>\d+(?:[.,]\d{1,5})?)(?::(?<S>\d+(?:[.,]\d{1,3}?)?))?)?$/;
    const match = regex.exec(value);
    if (!match) {
        return null;
    }
    const days = match.groups.d ? parseInt(match.groups.d) : 0;
    const hours = parseFloat(match.groups.H.replace(",", "."));
    const minutes = match.groups.M ? parseFloat(match.groups.M.replace(",", ".")) : 0.0;
    const seconds = match.groups.S ? parseFloat(match.groups.S.replace(",", ".")) : 0.0;
    const MILLISECONDS_PER_DAY = 86400000;
    const MILLISECONDS_PER_HOUR = 3600000;
    const MILLISECONDS_PER_MINUTE = 60000;
    const MILLISECONDS_PER_SECOND = 1000;
    return days * MILLISECONDS_PER_DAY
        + hours * MILLISECONDS_PER_HOUR
        + minutes * MILLISECONDS_PER_MINUTE
        + seconds * MILLISECONDS_PER_SECOND;
}

function timestamp_from_effective_date_and_time(naiveDate, naiveTime, effectiveBeginOfDayMilliseconds) {
    let result = new Date(naiveDate.getTime() + naiveTime.getTime());
    let beginIsAfterMidnight = naiveTime.getTime() < effectiveBeginOfDayMilliseconds;
    if (beginIsAfterMidnight) {
        result.setUTCDate(result.getUTCDate() + 1);
    }
    return result;
}

function formatDate(date) {
    return date.getUTCDate().toString().padStart(2, "0") + "." + (date.getUTCMonth() + 1).toString().padStart(2, "0") + ".";
}

function formatTime(date) {
    return date.getUTCHours().toString().padStart(2, "0")
        + ":" + (date.getUTCMinutes()).toString().padStart(2, "0")
        + (date.getUTCSeconds() ? ":" + (date.getUTCSeconds()).toString().padStart(2, "0") : "");
}

function ConcurrentEntriesFetcher(rooms, apiEndpoint, entryId, daySelect, beginInput, durationInput, roomsInput) {
    const SCHEDULE_TIMEOUT_MILLISECONDS = 300;
    const overlay = document.getElementById("concurrentEntriesOverlay");
    const spinner = document.getElementById("concurrentEntriesSpinner");
    const errorBox = document.getElementById("concurrentEntriesError");
    const resultsList = document.getElementById("concurrentEntriesList");
    const roomsMap = new Map(rooms.map((r) => [r.value, r.text]));

    let timeoutId = null;
    let abortController = null;

    this.doFetch = function() {
        activateSpinner();
        getConcurrentEntriesFromApi()
            .then((data) => {
                if (data === undefined) {
                    return;
                }
                console.debug("Retrieved concurrent entries: ", data);
                displayResult(data);
            });
    }

    async function getConcurrentEntriesFromApi() {
        const queryParameters = new URLSearchParams({
            effective_day: daySelect.value,
            begin_time: beginInput.value,
            duration: durationInput.value,
            rooms: roomsInput.value,
        });
        if (entryId !== null) {
            queryParameters.append("current_entry_id", entryId);
        }
        if (abortController !== null) {
            abortController.abort();
        }
        abortController = new AbortController();
        return window.fetch(apiEndpoint + "?" + queryParameters.toString(),
            {
                "signal": abortController.signal
            })
            .catch((err) => {
                if (err instanceof DOMException && err.name === "AbortError") {
                    console.log("Running fetch has been aborted");
                    return;
                }
                displayError(err.message);
                console.error("Failed to fetch concurrent entries: ", err);
            })
            .then(async (response) => {
                if (response.status === 422) {
                    displayError("Ungültige Eingabedaten");
                    console.warn("Failed to fetch concurrent entries: HTTP 422: " + await response.text());
                    return;
                } else if (!response.ok) {
                    displayError("Server-seitiger Fehler (HTTP " + response.status + ")");
                    console.warn("Failed to fetch concurrent entries: HTTP " + response.status + ": " + await response.text());
                    return;
                }
                return response.json();
            });
    }

    function activateSpinner() {
        errorBox.classList.add("d-none");
        spinner.classList.remove("d-none");
        overlay.classList.remove("d-none");
    }

    function displayResult(sortedEntries) {
        const selectedRooms = roomsInput.value.split(",");
        while(resultsList.firstChild) {
            resultsList.removeChild(resultsList.lastChild);
        }
        if (sortedEntries.length > 0) {
            for (const entry of sortedEntries) {
                resultsList.appendChild(generateResultRow(entry, selectedRooms));
            }
        } else {
            let infoRow = document.createElement("li");
            infoRow.classList.add("list-group-item", "text-info", "text-center");
            infoRow.innerText = "— Keine parallelen Einträge —";
            resultsList.appendChild(infoRow);
        }
        overlay.classList.add("d-none");
    }

    function generateResultRow(entry, selectedRooms) {
        let row = document.createElement("li");
        row.classList.add("list-group-item");
        if (entry.is_room_reservation) {
            row.classList.add("fst-italic");
        }
        let title = document.createElement("div");
        if (entry.is_exclusive) {
            let icon = document.createElement("i");
            icon.classList.add("bi", "bi-exclamation-diamond", "text-danger");
            icon.title = "Achtung: exklusiv";
            title.appendChild(icon);
            title.appendChild(document.createTextNode(" "));
        } else if (entry.has_room_conflict) {
            let icon = document.createElement("i");
            icon.classList.add("bi", "bi-exclamation-diamond", "text-warning");
            icon.title = "Achtung: Raum-Konflikt";
            title.appendChild(icon);
            title.appendChild(document.createTextNode(" "));
        }
        title.appendChild(document.createTextNode(entry.title));
        if (entry.is_exclusive) {
            title.appendChild(document.createTextNode(" "));
            let marker = document.createElement("span");
            marker.classList.add("text-danger", "fw-semibold");
            marker.innerText = "(exklusiv)";
            title.appendChild(marker);
        }
        row.appendChild(title);
        let roomInfo = document.createElement("small");
        roomInfo.classList.add("float-end");
        let firstRoom = true;
        for (const room of entry.rooms) {
            const isConflict = selectedRooms.includes(room);
            if (!firstRoom) {
                roomInfo.appendChild(document.createTextNode(", "));
            }
            let roomName = roomsMap.has(room) ? roomsMap.get(room) : "???";
            let roomSpan = document.createElement("span");
            if (isConflict) {
                roomSpan.classList.add("text-warning", "fw-semibold");
            }
            roomSpan.innerText = roomName;
            roomInfo.appendChild(roomSpan);
            firstRoom = false;
        }
        row.appendChild(roomInfo);
        let timeInfo = document.createElement("small");
        timeInfo.innerText = entry.begin + " – " + entry.end;
        row.appendChild(timeInfo);
        return row;
    }

    function displayError(error) {
        spinner.classList.add("d-none");
        errorBox.getElementsByClassName("error-message")[0].innerText = error;
        errorBox.classList.remove("d-none");
    }

    this.scheduleFetching = function () {
        if (!timeoutId !== null) {
            clearTimeout(timeoutId);
        }
        timeoutId = setTimeout(this.doFetch, SCHEDULE_TIMEOUT_MILLISECONDS);
    }
}
