import * as path from 'path';
import * as Mocha from 'mocha';
import {glob} from 'glob';

export function run(): Promise<void> {
    const reporter = process.env.MOCHA_REPORTER || 'spec';
    let reporterOptions = {};

    if (process.env.MOCHA_REPORTER_OPTIONS) {
        try {
            reporterOptions = JSON.parse(process.env.MOCHA_REPORTER_OPTIONS);
        } catch (err) {
            console.warn('Invalid MOCHA_REPORTER_OPTIONS JSON:', err);
        }
    }

	// Create the mocha test
	const mocha = new Mocha({
		ui: 'tdd',
        color: true,
        reporter,
        reporterOptions
	});

	const testsRoot = path.resolve(__dirname, '..');

	return new Promise((c, e) => {
		glob('**/**.test.js', { cwd: testsRoot }).then(files => {
			// Add files to the test suite
			files.forEach(f => mocha.addFile(path.resolve(testsRoot, f)));

			try {
				// Run the mocha test
				mocha.run(failures => {
					if (failures > 0) {
						e(new Error(`${failures} tests failed.`));
					} else {
						c();
					}
				});
			} catch (err) {
				console.error(err);
				e(err);
			}
		}).catch(err => {
			return e(err);
		});
	});
}
