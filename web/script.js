import * as styles from './style.css';
import 'htmx.org'

import * as echarts from 'echarts/core';
import { BarChart, PieChart } from 'echarts/charts';
import { SVGRenderer } from 'echarts/renderers';
import {
    TooltipComponent,
    GridComponent
} from 'echarts/components';

echarts.use([
    BarChart,
    PieChart,
    GridComponent,
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
    showInitialSubmissionDiagrams();
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
    } else if (value === 'nomvc') {
        Array.from(document.querySelectorAll('section.case')).forEach((section) => {
            section.style.display = 'none';
        })
        Array.from(document.querySelectorAll('section.case.nomvc')).forEach((section) => {
            section.style.display = 'block';
        })
    } else if (value === 'kdk_only') {
        Array.from(document.querySelectorAll('section.case')).forEach((section) => {
            section.style.display = 'none';
        })
        Array.from(document.querySelectorAll('section.case.kdk_only')).forEach((section) => {
            section.style.display = 'block';
        })
    } else if (value === 'grz_only') {
        Array.from(document.querySelectorAll('section.case')).forEach((section) => {
            section.style.display = 'none';
        })
        Array.from(document.querySelectorAll('section.case.grz_only')).forEach((section) => {
            section.style.display = 'block';
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
                    { name: 'Mit Fallnummer', value: data.cases['hnumber_case_count'], itemStyle: { color: '#4a5565' } },
                    { name: 'Ohne Fallnummer (aber aufgeklärt)', value: data.cases['case_count'] - data.cases['hnumber_case_count'], itemStyle: { color: '#eee' } }
                ],
            },
            {
                type: 'pie',
                radius: ['30%', '70%'],
                label: {
                    show: false,
                    position: 'center'
                },
                data: [
                    { name: 'Erforderliche Meldebestätigungen', value: data.submission_reports['required'], itemStyle: { color: '#016630' } },
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

export function showInitialSubmissionDiagrams() {
    Array.from(document.querySelectorAll('.initial-graph'))
        .map((elem) => elem.id)
        .forEach(showInitialSubmissionDiagram);
}

export function showInitialSubmissionDiagram(name) {
    let elem = document.getElementById(name);
    let data = JSON.parse(elem.dataset.value);

    if (!data) {
        return;
    }

    let chart = echarts.init(elem, null, {renderer: 'svg'});

    let months = data.kdk.map((item) => item.month);

    let option = {
        tooltip: {
            trigger: 'item',
            formatter: '{b}: {c} ({a})'
        },
        title: false,
        grid: {
            top: 8,
            bottom: 8,
            left: 0,
            right: 0,
            containLabel: true
        },
        xAxis: {
            data: months,
            axisLabel: {
                interval: 2
            }
        },
        yAxis: {
            show: false
        },
        series: [
            {
                name: 'KDK',
                type: 'bar',
                stack: 'total',
                label: {
                    show: false,
                    position: 'center'
                },
                itemStyle: {
                    color: '#707050'
                },
                data: data.kdk.map((item) => item.count)
            },
            {
                name: 'GRZ',
                type: 'bar',
                stack: 'total',
                label: {
                    show: false,
                    position: 'center'
                },
                itemStyle: {
                    color: '#505070'
                },
                data: data.grz.map((item) => item.count)
            }
        ]
    }

    option && chart.setOption(option);
}