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

window.addEventListener('load', formatTimeElements);