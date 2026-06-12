import * as styles from './style.css';
import 'htmx.org'

import * as echarts from 'echarts/core';
import { PieChart } from 'echarts/charts';
import { SVGRenderer } from 'echarts/renderers';
import {
    TooltipComponent
} from 'echarts/components';

echarts.use([
    PieChart,
    TooltipComponent,
    SVGRenderer
]);

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

const openAllCasesCheckbox = () => {
    document.querySelectorAll('section.case details').forEach((details) => {
        details.addEventListener('click', (event) => {
            document.getElementById('openAllCases').checked = false;
        });
    })
}

window.addEventListener('load', () => {
    formatTimeElements();
    openAllCasesCheckbox();
});

window.addEventListener('htmx:afterRequest', () => {
    formatTimeElements();
    openAllCasesCheckbox();
    showCasesDiagram();
});

window.addEventListener('htmx:responseError', (event) => {
    window.location.reload();
});

window.addEventListener('htmx:loadError', (event) => {
    window.location.reload();
});

export function changeVisibility(value) {
    Array.from(document.querySelectorAll('section.case')).forEach((section) => {
        section.style.display = 'block';
    });

    if (value === 'open') {
        Array.from(document.querySelectorAll('section.case.valid')).forEach((section) => {
            section.style.display = 'none';
        });
        Array.from(document.querySelectorAll('section.case.noh')).forEach((section) => {
            section.style.display = 'none';
        })
    } else if (value === 'withh') {
        Array.from(document.querySelectorAll('section.case.noh')).forEach((section) => {
            section.style.display = 'none';
        })
    }
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

export function showCasesDiagram() {
    let elem = document.getElementById('cases-graph');
    let data = JSON.parse(elem.dataset.value);

    if (!data) {
        return;
    }

    console.log(data);

    let chart = echarts.init(elem, null, {renderer: 'svg'});

    let option = {
        tooltip: {
            trigger: 'item'
        },
        title: false,
        series: [
            {
                type: 'pie',
                radius: ['72%', '80%'],
                label: {
                    show: false,
                    position: 'center'
                },
                data: [
                    { name: 'Mit Fallnummer', value: data.cases['hnumber_case_count'], itemStyle: { color: '#555' } },
                    { name: 'Ohne Fallnummer (aber aufgeklärt)', value: data.cases['case_count'] - data.cases['hnumber_case_count'], itemStyle: { color: '#eee' } }
                ],
            },
            {
                type: 'pie',
                radius: ['40%', '70%'],
                label: {
                    show: false,
                    position: 'center'
                },
                data: [
                    { name: 'Beide Meldebestätigungen', value: data.submission_reports['both'], itemStyle: { color: '#016630' } },
                    { name: 'Meldebestätigung nur vom KDK', value: data.submission_reports['kdk_only'], itemStyle: { color: '#d08700' } },
                    { name: 'Meldebestätigung nur vom GRZ', value: data.submission_reports['grz_only'], itemStyle: { color: '#d08700' } },
                    { name: 'Keine Meldebestätigung', value: data.submission_reports['missing_ongoing'], itemStyle: { color: '#9f0712' } },
                    { name: 'Ohne Fallnummer (aber aufgeklärt)', value: data.cases['case_count'] - data.cases['hnumber_case_count'], itemStyle: { color: '#eee' } }
                ],
            }
        ]
    }

    option && chart.setOption(option);
}