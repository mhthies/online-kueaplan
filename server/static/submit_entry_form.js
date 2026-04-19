function initializeSubmitEntryForm(effectiveBeginOfDayMilliseconds, rooms, concurrentEntriesApiEndpoint) {
    const daySelect = document.getElementById("daySelect");
    const beginInput = document.getElementById("beginInput");
    const durationInput = document.getElementById("durationInput");
    const endInput = document.getElementById("endInput");
    const roomsInput = document.getElementById("roomsInput");

    const calendarDateInfoElement = createCalendarDateInfoElement(beginInput);

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

    daySelect.addEventListener("input", () => {
        const naiveBeginDate = readDateSelect(daySelect);
        const naiveBeginTime = readNaiveTimeInput(beginInput);
        const durationMilliseconds = readNiceDurationInput(durationInput);
        updateCalendarDateInfo(calendarDateInfoElement, effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime);
        updateEndTimeInfo(endInput, effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime, durationMilliseconds);
        concurrentEntriesFetcher.scheduleFetching();
    });
    beginInput.addEventListener("input", () => {
        const naiveBeginDate = readDateSelect(daySelect);
        const naiveBeginTime = readNaiveTimeInput(beginInput);
        const durationMilliseconds = readNiceDurationInput(durationInput);
        updateCalendarDateInfo(calendarDateInfoElement, effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime);
        updateEndTimeInfo(endInput, effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime, durationMilliseconds);
        concurrentEntriesFetcher.scheduleFetching();
    });
    durationInput.addEventListener("input", () => {
        const naiveBeginDate = readDateSelect(daySelect);
        const naiveBeginTime = readNaiveTimeInput(beginInput);
        const durationMilliseconds = readNiceDurationInput(durationInput);
        updateEndTimeInfo(endInput, effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime, durationMilliseconds);
        concurrentEntriesFetcher.scheduleFetching();
    });
    roomsInput.addEventListener("input", () => {
        concurrentEntriesFetcher.scheduleFetching();
    });

    const naiveBeginDate = readDateSelect(daySelect);
    const naiveBeginTime = readNaiveTimeInput(beginInput);
    const durationMilliseconds = readNiceDurationInput(durationInput);
    updateCalendarDateInfo(calendarDateInfoElement, effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime);
    updateEndTimeInfo(endInput, effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime, durationMilliseconds);
    concurrentEntriesFetcher.doFetch();
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

function updateEndTimeInfo(endInput, effectiveBeginOfDayMilliseconds, naiveEffectiveBeginDate, naiveBeginTime, durationMilliseconds) {
    if (naiveEffectiveBeginDate === null || naiveBeginTime === null || durationMilliseconds === null) {
        endInput.value = "???";
        return;
    }

    const endDate = new Date(
        timestamp_from_effective_date_and_time(
            naiveEffectiveBeginDate, naiveBeginTime, effectiveBeginOfDayMilliseconds).getTime()
        + durationMilliseconds);
    const displayEndDate = (endDate.getUTCDate() !== naiveEffectiveBeginDate.getUTCDate()
        || endDate.getUTCMonth() !== naiveEffectiveBeginDate.getUTCMonth()
        || endDate.getUTCFullYear() !== naiveEffectiveBeginDate.getUTCFullYear());
    endInput.value = (displayEndDate ? formatDate(endDate) + " " : "") + formatTime(endDate);
}
