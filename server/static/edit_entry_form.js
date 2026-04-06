function initializeEditEntryForm(effectiveBeginOfDayMilliseconds, rooms, concurrentEntriesApiEndpoint, entryId) {
    const daySelect = document.getElementById("daySelect");
    const beginInput = document.getElementById("beginInput");
    const durationInput = document.getElementById("durationInput");
    const roomsInput = document.getElementById("roomsInput");

    const calendarDateInfoElement = createCalendarDateInfoElement(beginInput);
    const endTimeInfoElement = createEndTimeInfoElement(durationInput.parentElement);

    const concurrentEntriesFetcher = new ConcurrentEntriesFetcher(
        document.getElementById("parallelEntriesBox"),
        rooms,
        concurrentEntriesApiEndpoint,
        entryId,
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
        updateEndTimeInfo(endTimeInfoElement, effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime, durationMilliseconds);
        concurrentEntriesFetcher.scheduleFetching();
    });
    beginInput.addEventListener("input", () => {
        const naiveBeginDate = readDateSelect(daySelect);
        const naiveBeginTime = readNaiveTimeInput(beginInput);
        const durationMilliseconds = readNiceDurationInput(durationInput);
        updateCalendarDateInfo(calendarDateInfoElement, effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime);
        updateEndTimeInfo(endTimeInfoElement, effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime, durationMilliseconds);
        concurrentEntriesFetcher.scheduleFetching();
    });
    durationInput.addEventListener("input", () => {
        const naiveBeginDate = readDateSelect(daySelect);
        const naiveBeginTime = readNaiveTimeInput(beginInput);
        const durationMilliseconds = readNiceDurationInput(durationInput);
        updateEndTimeInfo(endTimeInfoElement, effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime, durationMilliseconds);
        concurrentEntriesFetcher.scheduleFetching();
    });
    roomsInput.addEventListener("input", () => {
        concurrentEntriesFetcher.scheduleFetching();
    });

    const naiveBeginDate = readDateSelect(daySelect);
    const naiveBeginTime = readNaiveTimeInput(beginInput);
    const durationMilliseconds = readNiceDurationInput(durationInput);
    updateCalendarDateInfo(calendarDateInfoElement, effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime);
    updateEndTimeInfo(endTimeInfoElement, effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime, durationMilliseconds);
    concurrentEntriesFetcher.doFetch();

    const orgaCommentInput = document.getElementById("orga_commentInput");
    orgaCommentInput.addEventListener("input", (e) => updateOrgaCommentBackground(e.target));
    updateOrgaCommentBackground(orgaCommentInput)

    const previousDateCheckbox = document.getElementById("create_previous_dateCheckbox");
    const previousDateCommentInput = document.getElementById("previous_date_commentInput");
    if (previousDateCheckbox && previousDateCommentInput) {
        previousDateCheckbox.addEventListener("change", (e) => {
            updatePreviousDateCommentState(e.target, previousDateCommentInput);
        });
        updatePreviousDateCommentState(previousDateCheckbox, previousDateCommentInput);
    }

}

function updateOrgaCommentBackground(orgaCommentInput) {
    if (orgaCommentInput.value === "") {
        orgaCommentInput.classList.remove("non-empty-comment");
    } else {
        orgaCommentInput.classList.add("non-empty-comment");
    }
}

function updatePreviousDateCommentState(previousDateCheckbox, previousDateCommentInput) {
    previousDateCommentInput.disabled = !previousDateCheckbox.checked;
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
        calendarDateInfoElement.getElementsByClassName("value")[0].innerText = formatDate(nextDay);
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
    icon.title = "Kalendertag";
    const iconDescription = document.createElement("span");
    iconDescription.classList.add("visually-hidden");
    iconDescription.innerText = "Kalendertag:";
    const text = document.createElement("span");
    text.classList.add("value");
    element.appendChild(icon);
    element.appendChild(iconDescription);
    element.appendChild(document.createTextNode(" "));
    element.appendChild(text)
    insertAfterElement.after(element);
    return element;
}

function updateEndTimeInfo(endTimeInfoElement, effectiveBeginOfDayMilliseconds, naiveEffectiveBeginDate, naiveBeginTime, durationMilliseconds) {
    if (naiveEffectiveBeginDate === null || naiveBeginTime === null || durationMilliseconds === null) {
        endTimeInfoElement.getElementsByTagName("span")[0].innerText = "???";
        return;
    }

    const endDate = new Date(
        timestamp_from_effective_date_and_time(
            naiveEffectiveBeginDate, naiveBeginTime, effectiveBeginOfDayMilliseconds).getTime()
        + durationMilliseconds);
    const displayEndDate = (endDate.getUTCDate() !== naiveEffectiveBeginDate.getUTCDate()
        || endDate.getUTCMonth() !== naiveEffectiveBeginDate.getUTCMonth()
        || endDate.getUTCFullYear() !== naiveEffectiveBeginDate.getUTCFullYear());
    endTimeInfoElement.getElementsByTagName("span")[0].innerText =
        (displayEndDate ? formatDate(endDate) + " " : "") + formatTime(endDate);
}

function createEndTimeInfoElement(insertAfterElement) {
    let element = document.createElement("div");
    element.setAttribute("aria-live", "polite");
    element.classList.add("form-text");
    element.id = "endTimeInfo";
    const text = document.createElement("span");
    element.appendChild(document.createTextNode("Ende: "));
    element.appendChild(text)
    insertAfterElement.after(element);
    return element;
}
