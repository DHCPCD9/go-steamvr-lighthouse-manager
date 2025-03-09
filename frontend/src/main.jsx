import {render} from 'preact';
import './localization/i18n'
import {App} from './app';
import './style.css';

render(<App/>, document.getElementById('app'));