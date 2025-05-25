function initializeEditEntryForm(effectiveBeginOfDayMilliseconds) {
    const daySelect = document.getElementById("daySelect");
    const beginInput = document.getElementById("beginInput");
    const durationInput = document.getElementById("durationInput");

    daySelect.addEventListener("input", () => {
        const naiveBeginDate = getNaiveSelectedDate();
        const naiveBeginTime = getNaiveBeginTime();
        const durationMilliseconds = getDurationMilliseconds();
        updateCalendarDateInfo(effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime);
        updateEndTimeInfo(effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime, durationMilliseconds);
    });
    beginInput.addEventListener("input", () => {
        const naiveBeginDate = getNaiveSelectedDate();
        const naiveBeginTime = getNaiveBeginTime();
        const durationMilliseconds = getDurationMilliseconds();
        updateCalendarDateInfo(effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime);
        updateEndTimeInfo(effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime, durationMilliseconds);
    });
    durationInput.addEventListener("input", () => {
        const naiveBeginDate = getNaiveSelectedDate();
        const naiveBeginTime = getNaiveBeginTime();
        const durationMilliseconds = getDurationMilliseconds();
        updateEndTimeInfo(effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime, durationMilliseconds);
    });

    const naiveBeginDate = getNaiveSelectedDate();
    const naiveBeginTime = getNaiveBeginTime();
    const durationMilliseconds = getDurationMilliseconds();
    updateCalendarDateInfo(effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime);
    updateEndTimeInfo(effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime, durationMilliseconds);
}

function updateCalendarDateInfo(effectiveBeginOfDayMilliseconds, naiveBeginDate, naiveBeginTime) {
    const calendarDateInfoElement = createOrGetCalendarDateInfoElement();

    if (naiveBeginDate === null || naiveBeginTime === null) {
        calendarDateInfoElement.classList.add("d-none");
        return;
    }

    let nextDay = new Date(naiveBeginDate);
    nextDay.setUTCDate(nextDay.getUTCDate() + 1);
    let beginIsAfterMidnight = naiveBeginTime.getTime() < effectiveBeginOfDayMilliseconds;
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
        element.appendChild(document.createTextNode(" "));
        element.appendChild(text)
        input.parentElement.insertBefore(element, null);
    }

    return element;
}

function updateEndTimeInfo(effectiveBeginOfDayMilliseconds, naiveEffectiveBeginDate, naiveBeginTime, durationMilliseconds) {
    let endTimeInfoElement = createOrGetEndTimeInfoElement();

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

function createOrGetEndTimeInfoElement() {
    let element = document.getElementById("endTimeInfo");

    if (!element) {
        const input = document.getElementById("durationInput");
        element = document.createElement("div");
        element.classList.add("form-text");
        element.id = "endTimeInfo";
        const text = document.createElement("span");
        element.appendChild(document.createTextNode("Ende: "));
        element.appendChild(text)
        // durationInput is wrapped in an input group
        input.parentElement.parentElement.insertBefore(element, null);
    }

    return element;
}

function getNaiveSelectedDate() {
    const daySelect = document.getElementById("daySelect");
    const date = new Date(daySelect.value);
    return isNaN(date) ? null : date;
}

function getNaiveBeginTime() {
    // All times are local time/naive/without timezone, but JavaScript does not support this concept.
    // So, we act like all times are UTC to avoid any conversion.
    const beginInput = document.getElementById("beginInput");
    const time = new Date("1970-01-01T" + beginInput.value + "Z");
    return isNaN(time) ? null : time;
}

function getDurationMilliseconds() {
    const durationInput = document.getElementById("durationInput");
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
