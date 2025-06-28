import {render} from 'preact';
import './localization/i18n'
import {App} from './app';
//@ts-ignore
import './style.css';

render(<App/>, document.getElementById('app'));