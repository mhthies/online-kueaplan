function initializeSubmitEntryForm(
    effectiveBeginOfDayMilliseconds,
    rooms,
    concurrentEntriesApiEndpoint,
    markdownPreviewApiEndpoint
) {
    const daySelect = document.getElementById("daySelect");
    const beginInput = document.getElementById("beginInput");
    const durationInput = document.getElementById("durationInput");
    const endInput = document.getElementById("endInput");
    const roomsInput = document.getElementById("roomsInput");
    const timePreview = document.getElementById("timePreview");
    const timePreview2 = document.getElementById("timePreview2");
    const roomPreview = document.getElementById("roomPreview");
    const roomPreview2 = document.getElementById("roomPreview2");
    const entryPreviewRow = document.getElementById("entryPreviewRow");
    const categorySelect = document.getElementById("categorySelect");
    const descriptionInput = document.getElementById("descriptionInput");
    const descriptionMarkdownPreview = document.getElementById("descriptionMarkdownPreview");

    const calendarDateInfoElement = createCalendarDateInfoElement(beginInput);

    const roomsMap = new Map(rooms.map((r) => [r.value, r.text]));
    const concurrentEntriesFetcher = new ConcurrentEntriesFetcher(
        document.getElementById("parallelEntriesBox"),
        rooms,
        concurrentEntriesApiEndpoint,
        null,
        daySelect,
        beginInput,
        durationInput,
        roomsInput
    );
    const markdownPreviewFetcher = new MarkdownPreviewLoader(
        descriptionMarkdownPreview,
        descriptionInput,
        markdownPreviewApiEndpoint
    );

    daySelect.addEventListener("input", () => {
        const naiveBeginDate = readDateSelect(daySelect);
        const naiveBeginTime = readNaiveTimeInput(beginInput);
        const durationMilliseconds = readNiceDurationInput(durationInput);
        updateCalendarDateInfo(calendarDateInfoElement, effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime);
        updateEndTimeInfoAndTimePreview(endInput, timePreview, timePreview2, effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime, durationMilliseconds);
        concurrentEntriesFetcher.scheduleFetching();
    });
    beginInput.addEventListener("input", () => {
        const naiveBeginDate = readDateSelect(daySelect);
        const naiveBeginTime = readNaiveTimeInput(beginInput);
        const durationMilliseconds = readNiceDurationInput(durationInput);
        updateCalendarDateInfo(calendarDateInfoElement, effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime);
        updateEndTimeInfoAndTimePreview(endInput, timePreview, timePreview2, effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime, durationMilliseconds);
        concurrentEntriesFetcher.scheduleFetching();
    });
    durationInput.addEventListener("input", () => {
        const naiveBeginDate = readDateSelect(daySelect);
        const naiveBeginTime = readNaiveTimeInput(beginInput);
        const durationMilliseconds = readNiceDurationInput(durationInput);
        updateEndTimeInfoAndTimePreview(endInput, timePreview, timePreview2, effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime, durationMilliseconds);
        concurrentEntriesFetcher.scheduleFetching();
    });
    roomsInput.addEventListener("input", () => {
        concurrentEntriesFetcher.scheduleFetching();
        updateRoomPreview(roomPreview, roomPreview2, roomsMap, roomsInput.value);
    });
    categorySelect.addEventListener("change", () => {
        updateCategoryPreview(entryPreviewRow, categorySelect.value);
    });
    descriptionInput.addEventListener("input", () => {
        markdownPreviewFetcher.scheduleFetching();
    })

    const naiveBeginDate = readDateSelect(daySelect);
    const naiveBeginTime = readNaiveTimeInput(beginInput);
    const durationMilliseconds = readNiceDurationInput(durationInput);
    updateCalendarDateInfo(calendarDateInfoElement, effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime);
    updateEndTimeInfoAndTimePreview(endInput, timePreview, timePreview2, effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime, durationMilliseconds);
    concurrentEntriesFetcher.doFetch();
    markdownPreviewFetcher.doFetch();
    updateRoomPreview(roomPreview, roomPreview2, roomsMap, roomsInput.value);
    updateCategoryPreview(entryPreviewRow, categorySelect.value);

    document.querySelectorAll("[data-copy-from]").forEach((e) => {
        const dataSource = document.getElementById(e.getAttribute("data-copy-from"));
        dataSource.addEventListener("input", () => { e.innerText = dataSource.value; });
        e.innerText = dataSource.value;
    });
}

function updateCalendarDateInfo(calendarDateInfoElement, effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime) {
    if (naiveBeginDate === null || naiveBeginTime === null) {
        calendarDateInfoElement.classList.add("d-none");
        return;
    }

    let nextDay = new Date(naiveBeginDate);
    nextDay.setUTCDate(nextDay.getUTCDate() + 1);
    let beginIsAfterMidnight = naiveBeginTime.getTime() < effectiveBeginOfDayMilliseconds;
    if (beginIsAfterMidnight) {
        calendarDateInfoElement.classList.remove("d-none");
        calendarDateInfoElement.getElementsByTagName("span")[0].innerText = formatDate(nextDay) + " " + formatTime(naiveBeginTime);
    } else {
        calendarDateInfoElement.classList.add("d-none");
    }
}

function createCalendarDateInfoElement(insertAfterElement) {
    let element = document.createElement("div");
    element.classList.add("form-text", "d-none", "text-info");
    element.id = "calendarDateInfo";
    element.setAttribute("aria-live", "polite");
    const icon = document.createElement("i");
    icon.classList.add("bi", "bi-calendar-event-fill");
    icon.setAttribute("aria-hidden", "true");
    const text = document.createElement("span");
    element.appendChild(document.createTextNode("Beginn am nächsten Kalendertag:"));
    element.appendChild(document.createElement("br"));
    element.appendChild(icon);
    element.appendChild(document.createTextNode(" "));
    element.appendChild(text)
    insertAfterElement.after(element);
    return element;
}

function updateEndTimeInfoAndTimePreview(endInput, timePreview, timePreview2, effectiveBeginOfDayMilliseconds, naiveEffectiveBeginDate, naiveBeginTime, durationMilliseconds) {
    if (naiveEffectiveBeginDate === null || naiveBeginTime === null || durationMilliseconds === null) {
        endInput.value = "???";
        return;
    }

    const startTimestamp = timestamp_from_effective_date_and_time(
        naiveEffectiveBeginDate, naiveBeginTime, effectiveBeginOfDayMilliseconds).getTime()
    const endDate = new Date(startTimestamp + durationMilliseconds);
    const displayEndDate = (endDate.getUTCDate() !== naiveEffectiveBeginDate.getUTCDate()
        || endDate.getUTCMonth() !== naiveEffectiveBeginDate.getUTCMonth()
        || endDate.getUTCFullYear() !== naiveEffectiveBeginDate.getUTCFullYear());
    endInput.value = (displayEndDate ? formatDate(endDate) + " " : "") + formatTime(endDate);
    timePreview.innerText = formatTime(naiveBeginTime) + " – " + formatTime(endDate);
    timePreview2.innerText = formatDate(new Date(startTimestamp)) + " " + formatTime(naiveBeginTime);
}

function updateRoomPreview(roomPreview, roomPreview2, roomsMap, selectedRoomIds) {
    if (selectedRoomIds.length === 0) {
        roomPreview.innerText = "";
        roomPreview2.innerText = "";
        return;
    }
    const roomIds = selectedRoomIds.split(",");
    const roomsString = roomIds.map((rid) => roomsMap.has(rid) ? roomsMap.get(rid) : "???").join(", ");
    roomPreview.innerText = roomsString;
    roomPreview2.innerText = " • " + roomsString;
}

function updateCategoryPreview(entryPreview, selectedCategoryId) {
    entryPreview.className = "kuea-with-category";
    entryPreview.classList.add("category-" + selectedCategoryId);
}

function MarkdownPreviewLoader(element, markdownInput, apiEndpoint) {
    const SCHEDULE_TIMEOUT_MILLISECONDS = 700;
    const resultBody = element.getElementsByClassName("body")[0];

    let timeoutId = null;
    let abortController = null;

    this.doFetch = function() {
        if (markdownInput.value === "") {
            element.classList.add("d-none");
            return;
            if (abortController !== null) {
                abortController.abort();
            }
        }
        activateSpinner();
        getMarkdownPreviewFromApi()
            .then((data) => {
                if (data === undefined) {
                    return;
                }
                resultBody.innerHTML = data;
            });
    }

    async function getMarkdownPreviewFromApi() {
        if (abortController !== null) {
            abortController.abort();
        }
        abortController = new AbortController();
        return window.fetch(apiEndpoint,
            {
                "signal": abortController.signal,
                "body": markdownInput.value,
                "method": "POST",
            })
            .catch((err) => {
                if (err instanceof DOMException && err.name === "AbortError") {
                    console.log("Running fetch has been aborted");
                    return;
                }
                displayError(err.message);
                console.error("Failed to fetch markdown preview: ", err);
            })
            .then(async (response) => {
                if (!response.ok) {
                    displayError("Server-seitiger Fehler (HTTP " + response.status + ")");
                    console.warn("Failed to fetch markdown preview: HTTP " + response.status + ": " + await response.text());
                    return;
                }
                return response.text();
            });
    }

    function activateSpinner() {
        element.classList.remove("d-none");
        resultBody.innerHTML = "<span class=\"spinner-border spinner-border-sm\" aria-hidden=\"true\"></span>\n"
            + "  <span class=\"visually-hidden\" role=\"status\">Lade...</span>";
    }

    function displayError(error) {
        resultBody.innerHTML = "";
        let text = document.createElement("span");
        text.classList.add("text-danger");
        text.innerText = error;
        resultBody.appendChild(text);
    }

    this.scheduleFetching = function () {
        if (!timeoutId !== null) {
            clearTimeout(timeoutId);
        }
        timeoutId = setTimeout(this.doFetch, SCHEDULE_TIMEOUT_MILLISECONDS);
    }
}
