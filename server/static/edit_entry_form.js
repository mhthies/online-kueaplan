function initializeEditEntryForm(effectiveBeginOfDayMilliseconds) {
    const daySelect = document.getElementById("daySelect");
    const beginInput = document.getElementById("beginInput");

    daySelect.addEventListener("change", () => {
        updateCalendarDateInfo(effectiveBeginOfDayMilliseconds);
    });
    beginInput.addEventListener("change", () => {
        updateCalendarDateInfo(effectiveBeginOfDayMilliseconds);
    });

    updateCalendarDateInfo(effectiveBeginOfDayMilliseconds);
}

function updateCalendarDateInfo(effectiveBeginOfDayMilliseconds) {
    const daySelect = document.getElementById("daySelect");
    const beginInput = document.getElementById("beginInput");
    const calendarDateInfoElement = createOrGetCalendarDateInfoElement();

    const dateMilliseconds = Date.parse(daySelect.value);
    // All times are local time, but since we do no complicated math here, we act like all times are UTC to avoid
    // conversion.
    const timeMilliseconds = Date.parse("1970-01-01T" + beginInput.value + "Z");
    if (isNaN(dateMilliseconds) || isNaN(timeMilliseconds)) {
        calendarDateInfoElement.classList.add("d-none");
        return;
    }
    const MILLISECONDS_PER_DAY = 86400000;

    let nextDay = new Date(dateMilliseconds + 1 * MILLISECONDS_PER_DAY);
    let beginIsAfterMidnight = timeMilliseconds < effectiveBeginOfDayMilliseconds;
    if (beginIsAfterMidnight) {
        calendarDateInfoElement.classList.remove("d-none");
        calendarDateInfoElement.getElementsByTagName("span")[0].innerText = formatDate(nextDay);
    } else {
        calendarDateInfoElement.classList.add("d-none");
    }
}

function createOrGetCalendarDateInfoElement() {
    let element = document.getElementById("calendarDateInfo");

    if (!element) {
        const input = document.getElementById("beginInput");
        element = document.createElement("div");
        element.classList.add("form-text", "d-none", "text-info");
        element.id = "calendarDateInfo";
        const icon = document.createElement("i");
        icon.classList.add("bi", "bi-calendar-event-fill");
        icon.title = "Kalendertag";
        const text = document.createElement("span");
        element.appendChild(icon);
        element.appendChild(document.createTextNode(" "));
        element.appendChild(text)
        input.parentElement.insertBefore(element, null);
    }

    return element;
}

function formatDate(date) {
    return date.getDate().toString().padStart(2, "0") + "." + (date.getMonth() + 1).toString().padStart(2, "0") + ".";
}
