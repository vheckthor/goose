import { datadogRum } from '@datadog/browser-rum';

export const initDatadog = () => {
    datadogRum.init({
        applicationId: process.env.DATADOG_APPLICATION_ID,
        clientToken: process.env.DATADOG_CLIENT_TOKEN,
        site: 'datadoghq.com',
        service: 'goose',
        env: process.env.DATADOG_ENV || 'dev',
        sessionSampleRate: 100,
        sessionReplaySampleRate: 20,
        trackUserInteractions: true,
        trackResources: true,
        trackLongTasks: true,
        defaultPrivacyLevel: 'mask-user-input',
    });
};