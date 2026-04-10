import * as styles from './style.css';

const dateTimeFormatOptions = { year: 'numeric', month: '2-digit', day: '2-digit' };
const dateTimeFormat = new Intl.DateTimeFormat('de-DE', dateTimeFormatOptions);

const formatTimeElements = () => {
    Array.from(document.getElementsByTagName('time')).forEach((timeTag) => {
        let date = Date.parse(timeTag.getAttribute('datetime'));
        if (! Number.isNaN(date)) {
            timeTag.innerText = dateTimeFormat.format(date);
        }
    });
};

window.addEventListener('load', () => {
    formatTimeElements();
    document.querySelectorAll('section.case details').forEach((details) => {
        details.addEventListener('click', (event) => {
            document.getElementById('openAllCases').checked = false;
        });
    })
});

export function hideCompletedCases(value) {
    Array.from(document.querySelectorAll('section.case.valid')).forEach((section) => {
        section.style.display = value === true ? 'none' : 'block';
    });
}

export function openAllCases(value) {
    if (value === true) {
        Array.from(document.querySelectorAll('section.case details')).forEach((details) => {
            details.setAttribute('open', '');
        });
    } else {
        Array.from(document.querySelectorAll('section.case.valid details')).forEach((details) => {
            details.removeAttribute('open');
        });
    }
}