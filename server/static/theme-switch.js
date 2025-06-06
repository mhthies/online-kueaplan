// Source: https://stackoverflow.com/a/75065536/10315508
// Set theme to the user's preferred color scheme
function updateTheme() {
    let colorMode = getCookie("theme");
    if (colorMode === null || colorMode === "auto") {
        colorMode = window.matchMedia("(prefers-color-scheme: dark)").matches ?
            "dark" :
            "light";
    }
    document.querySelector("html").setAttribute("data-bs-theme", colorMode);
}

// Set theme on load
updateTheme()

// Update theme when the preferred scheme changes
window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', updateTheme)

function setColorTheme(theme) {
    setCookie("theme", theme, 10*365);
}

function updateThemeButtonIcon(button) {
    const icon = button.getElementsByClassName("bi")[0];
    icon.classList.remove("bi-circle-half", "bi-moon", "bi-sun");
    let colorMode = getCookie("theme");
    if (colorMode === "dark") {
        icon.classList.add("bi-moon");
        button.title = "Style-Umschaltung. Aktuell: hell";
        button.setAttribute("data-next-theme", "light");
    } else if (colorMode === "light") {
        icon.classList.add("bi-sun");
        button.title = "Style-Umschaltung. Aktuell: dunkel";
        button.setAttribute("data-next-theme", "auto");
    } else {
        icon.classList.add("bi-circle-half");
        button.title = "Style-Umschaltung. Aktuell: System-abhängig";
        button.setAttribute("data-next-theme", "dark");
    }
}

function onThemeButtonClick(button) {
    setColorTheme(button.getAttribute("data-next-theme"));
    updateThemeButtonIcon(button);
    updateTheme();
}

function getCookie(cname) {
    // source: https://www.w3schools.com/js/js_cookies.asp
    let name = cname + "=";
    let decodedCookie = decodeURIComponent(document.cookie);
    let ca = decodedCookie.split(';');
    for(let i = 0; i <ca.length; i++) {
        let c = ca[i];
        while (c.charAt(0) == ' ') {
            c = c.substring(1);
        }
        if (c.indexOf(name) == 0) {
            return c.substring(name.length, c.length);
        }
    }
    return null;
}
function setCookie(cname, cvalue, exdays) {
    // source https://www.w3schools.com/js/js_cookies.asp
    const d = new Date();
    d.setTime(d.getTime() + (exdays*24*60*60*1000));
    let expires = "expires="+ d.toUTCString();
    document.cookie = cname + "=" + cvalue + ";" + expires + ";path=/";
}
